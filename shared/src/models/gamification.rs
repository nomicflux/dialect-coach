use crate::models::{LearningItemType, UserState};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GamificationStats {
    pub xp: u64,
    pub streak: StreakStats,
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct StreakStats {
    pub current_streak: u32,
    pub longest_streak: u32,
    pub last_active_date: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quest {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub dialect: crate::models::Dialect,
}

/// Derives the current gamification stats from the UserState on the fly.
pub fn derive_gamification_stats(state: &UserState) -> GamificationStats {
    GamificationStats {
        xp: calculate_xp(state),
        streak: calculate_streak(state),
        quests: derive_quests(state),
    }
}

fn calculate_xp(state: &UserState) -> u64 {
    state
        .learning_items
        .iter()
        .map(|item| item.score as u64)
        .sum()
}

fn calculate_streak(state: &UserState) -> StreakStats {
    let dates = get_user_activity_dates(state);
    let valid_days = get_valid_streak_days(dates);
    compute_streak_metrics(valid_days)
}

fn derive_quests(state: &UserState) -> Vec<Quest> {
    let mistakes = get_recent_mistakes(state);
    mistakes
        .into_iter()
        .take(3)
        .map(|(desc, completed, dialect)| Quest {
            id: format!("quest_{}", desc.replace(" ", "_")), // Simple ID generation
            description: format!("Fix: {}", desc),
            completed,
            dialect,
        })
        .collect()
}

fn get_recent_mistakes(state: &UserState) -> Vec<(String, bool, crate::models::Dialect)> {
    state
        .learning_items
        .iter()
        .filter_map(|item| {
            if let LearningItemType::Mistake(ref m) = item.item {
                Some((m.specific_mistake.clone(), item.score >= 10, item.dialect)) // Arbitrary completion threshold for now
            } else {
                None
            }
        })
        .collect()
}

fn get_user_activity_dates(state: &UserState) -> Vec<NaiveDate> {
    let mut dates: Vec<NaiveDate> = state
        .conversation_history
        .iter()
        .filter(|m| !m.is_agent())
        .map(|m| m.metadata.timestamp.date_naive())
        .collect();
    dates.sort();
    dates
}

fn get_valid_streak_days(dates: Vec<NaiveDate>) -> Vec<NaiveDate> {
    if dates.is_empty() {
        return Vec::new();
    }
    let daily_counts = count_daily_messages(dates);
    daily_counts
        .into_iter()
        .filter(|&(_, count)| count >= 5)
        .map(|(date, _)| date)
        .collect()
}

fn count_daily_messages(dates: Vec<NaiveDate>) -> Vec<(NaiveDate, usize)> {
    let mut counts = Vec::new();
    if dates.is_empty() {
        return counts;
    }

    let mut current_date = dates[0];
    let mut count = 0;

    for date in dates {
        if date == current_date {
            count += 1;
        } else {
            counts.push((current_date, count));
            current_date = date;
            count = 1;
        }
    }
    counts.push((current_date, count));
    counts
}

fn compute_streak_metrics(valid_days: Vec<NaiveDate>) -> StreakStats {
    let (current_streak, longest_streak) = calculate_streaks(&valid_days);
    let last_active = get_last_active_timestamp(&valid_days);

    StreakStats {
        current_streak,
        longest_streak,
        last_active_date: last_active,
    }
}

fn calculate_streaks(dates: &[NaiveDate]) -> (u32, u32) {
    let mut current_seq = 0;
    let mut max_seq = 0;
    let mut last_date: Option<NaiveDate> = None;

    for &date in dates {
        if let Some(prev) = last_date {
            if (date - prev).num_days() == 1 {
                current_seq += 1;
            } else {
                max_seq = max_seq.max(current_seq);
                current_seq = 1;
            }
        } else {
            current_seq = 1;
        }
        last_date = Some(date);
    }
    max_seq = max_seq.max(current_seq);

    let alive_streak = if is_streak_alive(last_date, current_seq) {
        current_seq
    } else {
        0
    };
    (alive_streak, max_seq)
}

fn is_streak_alive(last_date: Option<NaiveDate>, current_seq: u32) -> bool {
    let now = Utc::now().date_naive();
    match last_date {
        Some(last) => (now - last).num_days() <= 1 && current_seq > 0,
        None => false,
    }
}

fn get_last_active_timestamp(dates: &[NaiveDate]) -> i64 {
    dates
        .last()
        .map(|d| {
            DateTime::<Utc>::from_naive_utc_and_offset(
                d.and_time(chrono::NaiveTime::default()),
                Utc,
            )
            .timestamp()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Dialect, Formality, Language, Message, MessageMetadata, TeachingMode};
    use crate::models::{LearningItem, LearningItemType, Mistake};
    use chrono::Duration;
    use uuid::Uuid;

    fn mock_metadata(timestamp: DateTime<Utc>) -> MessageMetadata {
        MessageMetadata::new(
            Formality::Informal,
            TeachingMode::Immersive,
            Language::Spanish,
            Dialect::SpanishArgentinian,
            timestamp,
            Uuid::new_v4(),
        )
    }

    #[test]
    fn test_calculate_streak_simple() {
        let today = Utc::now();
        let yesterday = today - Duration::days(1);

        let mut state = UserState::new(Uuid::new_v4());

        for _ in 0..5 {
            state.conversation_history.push(Message::user_message(
                "msg".into(),
                mock_metadata(yesterday),
                None,
            ));
        }

        for _ in 0..5 {
            state.conversation_history.push(Message::user_message(
                "msg".into(),
                mock_metadata(today),
                None,
            ));
        }

        let stats = derive_gamification_stats(&state);
        assert_eq!(stats.streak.current_streak, 2);
    }

    #[test]
    fn test_calculate_streak_insufficient_messages() {
        let today = Utc::now();
        let mut state = UserState::new(Uuid::new_v4());
        for _ in 0..4 {
            state.conversation_history.push(Message::user_message(
                "msg".into(),
                mock_metadata(today),
                None,
            ));
        }
        let stats = derive_gamification_stats(&state);
        // Streak should be 0 because 4 messages < 5
        assert_eq!(stats.streak.current_streak, 0);
    }

    #[test]
    fn test_derive_quests() {
        let mut state = UserState::new(Uuid::new_v4());
        state.learning_items.push(LearningItem::new(
            LearningItemType::Mistake(Mistake::new(
                "error1".to_string(),
                "corr1".to_string(),
                crate::models::agent::MistakeCategory::SpellingError {
                    context: "cat1".to_string(),
                },
            )),
            Dialect::SpanishMexican,
        ));

        let stats = derive_gamification_stats(&state);
        assert_eq!(stats.quests.len(), 1);
        assert_eq!(stats.quests[0].description, "Fix: error1");
        assert_eq!(stats.quests[0].completed, false);
    }
}
