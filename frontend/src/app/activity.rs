use crate::app::app_callbacks::{chat_action, reply_origin, send_chat};
use crate::app::app_state::{
    ActivityOperation, AppState, AppStateAction, BranchAction, SessionAction, SessionState,
    UIState, UIStateAction, UserDomainAction,
};
use crate::services::persistence::StoredRequest;
use dialect_coach_shared::{
    AIActionRequest, ChatRequest, ClientMessage, EvaluateActivityRequest, GenerateActivityRequest,
    Reply, StudyRequest, UserState,
};
use gloo::timers::callback::Timeout;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub fn generate(
    app: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui: UseReducerHandle<UIState>,
) -> Callback<String> {
    Callback::from(move |description: String| {
        let Some(user) = &session.user else {
            return;
        };
        let branch_id = user.active_branch_id;
        if ui
            .activities
            .get(&branch_id)
            .is_some_and(|a| a.busy.is_some())
            || user
                .current_activity(branch_id)
                .is_some_and(|s| s.is_active())
        {
            return;
        }
        let Some(branch) = user.branches.iter().find(|b| b.id == branch_id) else {
            return;
        };
        let expected_head = branch.leaf_message_id;
        let messages = user
            .get_active_branch_messages()
            .into_iter()
            .cloned()
            .collect();
        let request = GenerateActivityRequest {
            user_id: user.user_id,
            branch_id,
            description: (!description.trim().is_empty()).then_some(description),
            context: user.build_action_context(messages),
        };
        ui.dispatch(UIStateAction::ActivityBusy(
            branch_id,
            ActivityOperation::Generate,
        ));
        let future = app
            .connection
            .request(ClientMessage::Study(StudyRequest::GenerateActivity(
                Box::new(request),
            )));
        let (session, ui) = (session.clone(), ui.clone());
        spawn_local(async move {
            match future.await {
                Ok(Reply::ActivityGenerated(Ok(definition))) => {
                    let id = definition.activity_id;
                    session.dispatch(SessionAction::Domain(UserDomainAction::Branch(
                        BranchAction::ActivityGenerated {
                            branch_id,
                            expected_head,
                            definition,
                        },
                    )));
                    ui.dispatch(UIStateAction::ActivityFinished(branch_id, None));
                    ui.dispatch(UIStateAction::ActivityInstalled(branch_id, id));
                }
                _ => ui.dispatch(UIStateAction::ActivityFinished(
                    branch_id,
                    Some(ActivityOperation::Generate),
                )),
            }
        });
    })
}

pub fn open(
    app: &UseReducerHandle<AppState>,
    session: &UseReducerHandle<SessionState>,
    ui: &UseReducerHandle<UIState>,
    branch_id: Uuid,
) {
    let Some(user) = &session.user else {
        return;
    };
    if ui
        .activities
        .get(&branch_id)
        .is_some_and(|a| a.busy.is_some())
    {
        return;
    }
    let Some(branch) = user.branches.iter().find(|b| b.id == branch_id) else {
        return;
    };
    let messages = branch
        .message_ids
        .iter()
        .filter_map(|id| user.msg_by_id(*id))
        .collect();
    let Some(context) = user.build_branch_action_context(branch_id, messages) else {
        return;
    };
    if !context
        .activity
        .as_ref()
        .is_some_and(|a| a.snapshot.is_active())
    {
        return;
    }
    let request = chat_action(
        app.session_id(),
        user.user_id,
        AIActionRequest::StartActivity {
            context: Box::new(context),
        },
    );
    spawn_local(send_chat(app, session, ui, StoredRequest::new(request)));
}

pub fn end(
    session: &UseReducerHandle<SessionState>,
    ui: &UseReducerHandle<UIState>,
    branch_id: Uuid,
    activity_id: Uuid,
) {
    commit_focused_goal();
    session.dispatch(SessionAction::Domain(UserDomainAction::Branch(
        BranchAction::EndActivity {
            branch_id,
            activity_id,
        },
    )));
    ui.dispatch(UIStateAction::ActivityEnded(branch_id));
}

/// Force the same onchange event used by ordinary goal editing before hiding its field.
pub fn commit_focused_goal() {
    use wasm_bindgen::JsCast;
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        && element.has_attribute("data-activity-goal")
        && let Some(element) = element.dyn_ref::<web_sys::HtmlElement>()
    {
        let _ = element.blur();
    }
}

fn check_token(user: &UserState, branch_id: Uuid, force: bool) -> Option<(Uuid, Option<Uuid>)> {
    let activity = user.current_activity(branch_id).filter(|s| s.is_active())?;
    if activity.definition.goals.is_empty() {
        return None;
    }
    let branch = user.branches.iter().find(|b| b.id == branch_id)?;
    let messages: Vec<_> = branch
        .message_ids
        .iter()
        .filter_map(|id| user.msg_by_id(*id))
        .collect();
    let context = user.activity_context(branch_id, &messages)?;
    if context.activity_message_ids.is_empty() {
        return None;
    }
    let last_user = messages
        .iter()
        .rev()
        .find(|m| !m.is_agent() && context.activity_message_ids.contains(&m.id))
        .map(|m| m.id);
    let checked = activity
        .last_checked_message_id
        .and_then(|id| branch.message_ids.iter().position(|m| *m == id));
    let user_index = last_user.and_then(|id| branch.message_ids.iter().position(|m| *m == id));
    let edited = branch
        .activity_snapshots
        .iter()
        .find(|s| s.definition.activity_id == activity.definition.activity_id)
        .is_some_and(|first| first.snapshot_id != activity.snapshot_id);
    if force
        || (activity.last_checked_message_id.is_none() && (last_user.is_some() || edited))
        || user_index > checked
    {
        Some((activity.snapshot_id, last_user))
    } else {
        None
    }
}

#[hook]
pub fn use_activity_requests(
    app: UseReducerHandle<AppState>,
    session: UseReducerHandle<SessionState>,
    ui: UseReducerHandle<UIState>,
) {
    {
        let ui = ui.clone();
        use_effect_with(ui.focus_activity_start, move |focus| {
            if *focus {
                use wasm_bindgen::JsCast;
                if let Some(button) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("activity-start"))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    let _ = button.focus();
                }
                ui.dispatch(UIStateAction::ClearActivityFocus);
            }
        });
    }
    let branch = session.user.as_ref().map(|u| u.active_branch_id);
    {
        let ui = ui.clone();
        use_effect_with(branch, move |branch| {
            if let Some(branch) = branch {
                ui.dispatch(UIStateAction::SelectBranch(*branch));
            }
        });
    }
    {
        let (app, ui) = (app.clone(), ui.clone());
        use_effect_with(ui.reply_effect.clone(), move |message| {
            if let Some(message) = message {
                app.dispatch(AppStateAction::ProcessAgentMessage(message.clone()));
                ui.dispatch(UIStateAction::ClearReplyEffect);
            }
        });
    }
    {
        let (app, session, ui) = (app.clone(), session.clone(), ui.clone());
        use_effect_with((session.clone(), ui.clone()), move |_| {
            for (branch_id, status) in &ui.activities {
                if let Some(id) = status.pending_open
                    && let Some(activity) = session
                        .user
                        .as_ref()
                        .and_then(|u| u.current_activity(*branch_id))
                    && activity.is_active()
                    && activity.definition.activity_id == id
                    && status.busy.is_none()
                {
                    ui.dispatch(UIStateAction::ActivityOpeningTaken(*branch_id));
                    open(&app, &session, &ui, *branch_id);
                }
            }
        });
    }
    let pending = session.user.as_ref().and_then(|user| {
        let id = user.active_branch_id;
        let status = ui.activities.get(&id);
        let token = check_token(user, id, status.is_some_and(|s| s.force_check))?;
        if status.is_some_and(|s| {
            s.busy.is_some() || s.pending_open.is_some() || s.attempted == Some(token)
        }) {
            return None;
        }
        Some((id, token))
    });
    use_effect_with(pending, move |pending| {
        let timer = pending.map(|(branch_id, token)| {
            Timeout::new(500, move || {
                let Some(user) = &session.user else {
                    return;
                };
                let Some(branch) = user.branches.iter().find(|b| b.id == branch_id) else {
                    return;
                };
                let messages = branch
                    .message_ids
                    .iter()
                    .filter_map(|id| user.msg_by_id(*id))
                    .collect();
                let Some(context) = user.build_branch_action_context(branch_id, messages) else {
                    return;
                };
                let chat = ChatRequest::Action {
                    session_id: app.session_id().unwrap_or_default(),
                    user_id: user.user_id,
                    action: Box::new(AIActionRequest::ContinueBranch {
                        parent_message_id: branch.leaf_message_id.unwrap_or_default(),
                        context: Box::new(context.clone()),
                    }),
                };
                let origin = reply_origin(&chat, branch_id, user);
                let request = EvaluateActivityRequest {
                    user_id: user.user_id,
                    context,
                };
                ui.dispatch(UIStateAction::ActivityAttempted(
                    branch_id, token.0, token.1,
                ));
                ui.dispatch(UIStateAction::ActivityBusy(
                    branch_id,
                    ActivityOperation::Check,
                ));
                let future =
                    app.connection
                        .request(ClientMessage::Study(StudyRequest::EvaluateActivity(
                            Box::new(request),
                        )));
                let (session, ui) = (session.clone(), ui.clone());
                spawn_local(async move {
                    match future.await {
                        Ok(Reply::ActivityEvaluated(Ok(analysis)))
                            if analysis.goal_completion.is_some() =>
                        {
                            session.dispatch(SessionAction::Domain(UserDomainAction::Branch(
                                BranchAction::ActivityChecked { origin, analysis },
                            )));
                            ui.dispatch(UIStateAction::ActivityFinished(branch_id, None));
                        }
                        _ => ui.dispatch(UIStateAction::ActivityFinished(
                            branch_id,
                            Some(ActivityOperation::Check),
                        )),
                    }
                });
            })
        });
        move || drop(timer)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app_state::user::reducer::{reduce_branch, reduce_learning, reduce_message};
    use crate::app::app_state::{GoalTarget, LearningAction, MessageAction};
    use dialect_coach_shared::{ActivityDefinition, AgentResponse, LearningGoal, Message};

    #[test]
    fn opening_does_not_trigger_second_check_but_edits_and_explicit_retry_do() {
        let mut user = UserState::new(Uuid::new_v4());
        let branch = user.active_branch_id;
        let old = user.create_user_msg(Uuid::new_v4(), "Earlier conversation");
        reduce_message(&mut user, MessageAction::Add(old.clone()));
        let definition = ActivityDefinition {
            activity_id: Uuid::new_v4(),
            title: "Meet".into(),
            shared_context: "Meet later".into(),
            learner_brief: "Choose a time".into(),
            partner_brief: "After five".into(),
            goals: vec![LearningGoal {
                goal: "Agree a time".into(),
                dialect: user.current_dialect(),
                completed: None,
            }],
        };
        let activity_id = definition.activity_id;
        reduce_branch(
            &mut user,
            BranchAction::ActivityGenerated {
                branch_id: branch,
                expected_head: Some(old.id),
                definition,
            },
        );
        assert!(check_token(&user, branch, false).is_none());
        let opening = Message::agent_message(
            AgentResponse::from("¿Cuándo?"),
            old.metadata.clone(),
            Some(old.id),
        );
        reduce_message(&mut user, MessageAction::Add(opening));
        assert!(check_token(&user, branch, false).is_none());
        assert!(check_token(&user, branch, true).is_some());
        reduce_learning(
            &mut user,
            LearningAction::UpdateGoal(
                GoalTarget::Activity {
                    branch_id: branch,
                    activity_id,
                },
                0,
                "Agree a time and place".into(),
            ),
        );
        assert!(check_token(&user, branch, false).is_some());
        let input = user.create_user_msg(Uuid::new_v4(), "A las seis");
        reduce_message(&mut user, MessageAction::Add(input.clone()));
        let messages = user
            .get_active_branch_messages()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let origin = crate::app::app_state::ReplyOrigin {
            branch_id: branch,
            dialect: user.current_dialect(),
            plan_id: None,
            activity: user.activity_context(branch, &messages).map(Box::new),
            checked_head: Some(input.id),
        };
        let mut response = AgentResponse::from("De acuerdo");
        let mut analysis = dialect_coach_shared::AgentAnalysis::new();
        analysis.goal_completion = Some(vec![true]);
        response.analysis = Some(analysis);
        let reply = Message::agent_message(response, input.metadata.clone(), Some(input.id));
        reduce_message(
            &mut user,
            MessageAction::Reply {
                message: reply,
                origin,
            },
        );
        assert!(
            check_token(&user, branch, false).is_none(),
            "new assistant confirmation waits for next user/check"
        );
        let ui = UIState::default()
            .apply_action(UIStateAction::ActivityFinished(
                branch,
                Some(ActivityOperation::Check),
            ))
            .apply_action(UIStateAction::ActivityRetryCheck(branch));
        assert!(
            check_token(&user, branch, ui.activities[&branch].force_check).is_some(),
            "explicit retry checks without another user message"
        );
        reduce_branch(&mut user, BranchAction::Create(input.id));
        assert!(
            check_token(&user, user.active_branch_id, false).is_some(),
            "fork at unchecked user input must check inherited prefix"
        );
        reduce_branch(&mut user, BranchAction::Switch(branch));
        reduce_branch(
            &mut user,
            BranchAction::EndActivity {
                branch_id: branch,
                activity_id,
            },
        );
        assert!(check_token(&user, branch, true).is_none());
    }
}
