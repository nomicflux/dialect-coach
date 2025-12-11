use crate::services::enrichment_service::EnrichmentService;
use dialect_coach_shared::models::{
    EnrichRequest, PartialLearningItem,
    plan::{LanguagePlan, import::ImportLanguagePlan},
};
use std::rc::Rc;

/// Orchestrates the entire import process from text to final plan.
pub async fn process_imported_text(
    text: String,
    enrichment_service: Rc<EnrichmentService>,
) -> Result<LanguagePlan, String> {
    let import_plan = parse_import_plan(&text)?;
    let mut hydrated = import_plan
        .hydrate()
        .map_err(|e| format!("Failed to hydrate plan: {}", e))?;

    let items = extract_enrichable_items(&hydrated);
    if !items.is_empty() {
        let enriched_data = enrich_items_parallel(items, enrichment_service).await;
        apply_enrichment_results(&mut hydrated, enriched_data);
    }

    hydrated
        .try_into_plan()
        .map_err(|e| format!("Failed to finalize plan: {}", e))
}

fn parse_import_plan(text: &str) -> Result<ImportLanguagePlan, String> {
    serde_yaml::from_str::<ImportLanguagePlan>(text)
        .map_err(|e| format!("Failed to parse YAML: {}", e))
}

/// Identifies incomplete items that need enrichment.
/// Returns (step_index, item_index, dialect, partial_item).
fn extract_enrichable_items(
    hydrated: &dialect_coach_shared::models::plan::import::HydratedLanguagePlan,
) -> Vec<(
    usize,
    usize,
    dialect_coach_shared::Dialect,
    PartialLearningItem,
)> {
    let mut items = Vec::new();
    for (s_idx, step) in hydrated.steps.iter().enumerate() {
        if let dialect_coach_shared::models::plan::import::HydratedStepType::Learning { content } =
            &step.step_type
        {
            for (i_idx, item) in content.items.iter().enumerate() {
                if !item.item.is_complete() {
                    items.push((s_idx, i_idx, item.dialect, item.item.clone()));
                }
            }
        }
    }
    items
}

/// Executes enrichment requests in parallel.
async fn enrich_items_parallel(
    items: Vec<(
        usize,
        usize,
        dialect_coach_shared::Dialect,
        PartialLearningItem,
    )>,
    service: Rc<EnrichmentService>,
) -> Vec<(usize, usize, Result<PartialLearningItem, String>)> {
    let futures = items.iter().map(|(s_idx, i_idx, dialect, partial)| {
        let service = service.clone();
        let dialect = *dialect;
        let partial = partial.clone();
        let s_idx = *s_idx;
        let i_idx = *i_idx;
        async move {
            let req = EnrichRequest {
                dialect,
                partial_data: partial.clone(),
            };
            let result = match service.enrich_learning_item(req).await {
                Ok(resp) => {
                    let mut value = resp.enriched_item;
                    // Inject missing "type" field if needed
                    if let Some(obj) = value.as_object_mut() 
                        && !obj.contains_key("type")
                    {
                        let type_str = match &partial {
                            PartialLearningItem::Mistake(_) => "mistake",
                            PartialLearningItem::Explained(_) => "explained",
                            PartialLearningItem::Translated(_) => "translated",
                            PartialLearningItem::Exploratory(_) => "exploratory",
                        };
                        obj.insert(
                            "type".to_string(),
                            serde_json::Value::String(type_str.to_string()),
                        );
                    }
                    serde_json::from_value::<PartialLearningItem>(value)
                        .map_err(|e| format!("Parse error: {}", e))
                }
                Err(e) => Err(format!("Enrichment error: {}", e)),
            };
            (s_idx, i_idx, result)
        }
    });

    futures_util::future::join_all(futures).await
}

fn apply_enrichment_results(
    hydrated: &mut dialect_coach_shared::models::plan::import::HydratedLanguagePlan,
    results: Vec<(usize, usize, Result<PartialLearningItem, String>)>,
) {
    for (s_idx, i_idx, result) in results {
        if let Ok(enriched) = result {
            if let dialect_coach_shared::models::plan::import::HydratedStepType::Learning {
                content,
            } = &mut hydrated.steps[s_idx].step_type
                && let Some(item_ref) = content.items.get_mut(i_idx)
            {
                item_ref.item = enriched;
            }
        } else if let Err(e) = result {
            gloo::console::error!(&e);
        }
    }
}
