use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    response::Response,
    routing::post,
};
use base64::prelude::*;
use dialect_coach_backend::agent_service::{
    AgentService, activity, analysis::AnalysisRequestParams, keyword_extraction::KeywordExtractor,
    ocr::OcrService, planning::PlanGenerator, provider::CompletionAgentFactory,
    response::GenerateResponseParams,
};
use dialect_coach_backend::{
    embedding_service::TextEmbedder, parsing, planning_handler, qdrant_service::QdrantService,
    rag_config::RAGConfig,
};
use dialect_coach_shared::{
    Dialect, Formality, GenerateActivityRequest, Mistake, MistakeCategory, PlanRequest, PlanSource,
    TeachingMode, UserGender, UserState, config::LlmConfig, dialect_features,
};
use prost::Message;
use qdrant_client::qdrant::{
    HealthCheckReply, QueryPoints, QueryResponse, ScoredPoint, query, vector_input,
};
use rig::completion::Message as RigMessage;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const SCAN: &[u8] = include_bytes!("fixtures/two-page-scan.pdf");
const TRANSCRIPT: &str = "LINE ONE: BLUE BICYCLE\nLINE TWO: RED BANANA";
const INPUT: &str = "¿Dónde está la bicicleta?";
const REPLY: &str = "Está a la vuelta.";
const CORPUS_EXAMPLE: &str = "Da la vuelta en la esquina.";
const CORPUS_DIALECT: Dialect = Dialect::SpanishArgentinian;
const DIALECT: Dialect = Dialect::SpanishMexican;
const PLAN: &str = r#"title: "Scan plan"
dialect: spanish_mexican
steps:
  - title: "Vocabulary"
    instructions: "Learn the words from the scan"
    learning_content:
      - vocab: "bicicleta"
        translation: "bicycle"
"#;

#[derive(Default)]
struct Requests {
    llm: Mutex<Vec<Value>>,
    qdrant: Mutex<Vec<QueryPoints>>,
    score_id: Mutex<Option<uuid::Uuid>>,
}

#[derive(Default)]
struct RecordingEmbedder(Mutex<Vec<Vec<String>>>);

#[async_trait::async_trait]
impl TextEmbedder for RecordingEmbedder {
    async fn embed_batch(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
        let vectors = texts
            .iter()
            .map(|text| match text.as_str() {
                "alfa" => vec![1.0, 0.0],
                "zeta" => vec![0.0, 1.0],
                _ => vec![0.5, 0.5],
            })
            .collect();
        self.0.lock().unwrap().push(texts);
        Ok(vectors)
    }
}

async fn serve(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
    url
}

fn grpc(message: impl Message) -> Response {
    let encoded = message.encode_to_vec();
    let mut frame = vec![0];
    frame.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
    frame.extend(encoded);
    Response::builder()
        .header("content-type", "application/grpc")
        .header("grpc-status", "0")
        .body(Body::from(frame))
        .unwrap()
}

async fn query_corpus(State(state): State<Arc<Requests>>, bytes: Bytes) -> Response {
    let request = QueryPoints::decode(&bytes[5..]).unwrap();
    let result = if request.using.as_deref() == Some("content") {
        vec![ScoredPoint {
            score: 0.8,
            payload: [
                ("content".into(), CORPUS_EXAMPLE.into()),
                ("dialect".into(), CORPUS_DIALECT.id().into()),
                ("formality".into(), "Informal".into()),
            ]
            .into(),
            ..Default::default()
        }]
    } else {
        vec![]
    };
    state.qdrant.lock().unwrap().push(request);
    grpc(QueryResponse {
        result,
        ..Default::default()
    })
}

async fn llm(State(state): State<Arc<Requests>>, Json(body): Json<Value>) -> Json<Value> {
    let mut requests = state.llm.lock().unwrap();
    let model = body["model"].as_str().unwrap();
    let text = match model {
        "keywords" => r#"{"keywords":["zeta","alfa"]}"#.into(),
        "response" if requests.iter().all(|body| body["model"] != "response") => "malformed JSON".into(),
        "response" => json!({"response": REPLY}).to_string(),
        "learning" if messages(&body).last().unwrap().1.starts_with("USER MESSAGE:") => {
            r#"{"translated":[{"translated_word":"bicicleta","translated_to":"bicycle"}]}"#.into()
        }
        "learning" => r#"{"mistakes":[{"specific_mistake":"un bicicleta","correction":"una bicicleta","mistake_category":{"type":"grammar_error","context":"feminine noun"}}]}"#.into(),
        "pronunciation" => r#"{"pronunciation_text":"está a la vuelta"}"#.into(),
        "analysis" => json!({"scores": {state.score_id.lock().unwrap().unwrap().to_string(): 1}}).to_string(),
        "planning" if body["messages"][0]["content"].as_str().is_some_and(|prompt| prompt.contains("Author a short communicative activity")) => {
            r#"{"title":"Find the station","shared_context":"You are downtown","learner_brief":"Pregunta por la estación","partner_brief":"The station is on the left","goals":["Pide indicaciones","Confirma la dirección"]}"#.into()
        }
        "planning" => serde_yaml::from_str::<Value>(PLAN).unwrap().to_string(),
        "vision" => TRANSCRIPT.into(),
        _ => panic!("Unexpected model {model}"),
    };
    let reply = if body.get("system").is_some() {
        json!({
            "id": "msg_test", "type": "message", "model": model, "role": "assistant",
            "content": [{"type": "text", "text": text}], "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })
    } else {
        json!({
            "model": model, "created_at": "2026-01-01T00:00:00Z",
            "message": {"role": "assistant", "content": text},
            "done": true, "done_reason": "stop", "prompt_eval_count": 10, "eval_count": 5
        })
    };
    requests.push(body);
    Json(reply)
}

async fn llm_fixture(state: Arc<Requests>) -> String {
    serve(
        Router::new()
            .route("/api/chat", post(llm))
            .route("/v1/messages", post(llm))
            .layer(DefaultBodyLimit::disable())
            .with_state(state),
    )
    .await
}

fn config(url: &str, driver: &str) -> LlmConfig {
    serde_yaml::from_str(&format!(
        "
default_provider: fixture
providers:
  fixture:
    driver: {driver}
    base_url: {url}
    model: default
channels:
  response: {{ model: response }}
  keywords: {{ model: keywords }}
  planning: {{ model: planning }}
  ocr: {{ model: vision }}
  learning: {{ model: learning }}
  pronunciation: {{ model: pronunciation }}
  analysis: {{ model: analysis }}
",
    ))
    .unwrap()
}

fn messages(body: &Value) -> Vec<(&str, &str)> {
    body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| {
            let content = &message["content"];
            (
                message["role"].as_str().unwrap(),
                content
                    .as_str()
                    .or_else(|| content[0]["text"].as_str())
                    .unwrap(),
            )
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_retrieval_and_json_feature_routes() {
    let state = Arc::new(Requests::default());
    let url = llm_fixture(state.clone()).await;
    let qdrant_url = serve(
        Router::new()
            .route(
                "/qdrant.Qdrant/HealthCheck",
                post(|| async {
                    grpc(HealthCheckReply {
                        title: "fixture".into(),
                        version: "1.19.0".into(),
                        commit: None,
                    })
                }),
            )
            .route("/qdrant.Points/Query", post(query_corpus))
            .with_state(state.clone()),
    )
    .await;
    let embeddings = Arc::new(RecordingEmbedder::default());
    let qdrant = Arc::new(QdrantService::new(&qdrant_url, "").await.unwrap());
    let mut service =
        AgentService::from_config(&config(&url, "ollama"), qdrant, embeddings.clone()).unwrap();
    let dialect = dialect_features(CORPUS_DIALECT);
    assert!(dialect.has_corpus);
    let history = [
        RigMessage::user("Hola"),
        RigMessage::assistant("¿Qué necesitas?"),
    ];
    let params = GenerateResponseParams {
        activity: None,
        user_message: INPUT,
        dialect,
        formality: Formality::Informal,
        teaching_mode: TeachingMode::Corrective,
        conversation_history: &history,
        learning_goals: &[],
        rag_config: &RAGConfig::default_config(),
        past_mistakes: &[],
        past_explained: &[],
        past_translated: &[],
        past_exploratory: &[],
        user_gender: UserGender::NonBinary,
        language_option: &None,
        active_plan: None,
        language_level: Default::default(),
    };
    let mistake = Mistake::new(
        "un bicicleta".into(),
        "una bicicleta".into(),
        MistakeCategory::GrammarError {
            context: "feminine noun".into(),
        },
    );
    *state.score_id.lock().unwrap() = Some(mistake.id);
    let analysis_input = INPUT.to_string();
    let analyze = || AnalysisRequestParams {
        allowed: true,
        activity: None,
        dialect: CORPUS_DIALECT,
        msg: &analysis_input,
        mistakes: std::slice::from_ref(&mistake),
        explained: &[],
        translated: &[],
        exploratory: &[],
        language_option: &None,
    };

    let (reply, response_usage, _) = service.generate_response(&params).await;
    let reply = reply.unwrap();
    assert_eq!(reply.response, REPLY);
    assert_eq!(reply.mistakes.unwrap()[0].0.correction, "una bicicleta");
    assert_eq!(reply.translated.unwrap()[0].0.translated_to, "bicycle");
    assert_eq!(
        reply.pronunciation_text.as_deref(),
        Some("está a la vuelta")
    );
    assert_eq!(
        response_usage.len(),
        2,
        "malformed response must use the repair path"
    );
    let (analysis, _) = service.generate_analysis(analyze()).await;
    assert_eq!(analysis.unwrap().mistake_scores[&mistake.id].score, 1);
    let local = std::mem::take(&mut *state.llm.lock().unwrap());
    assert_eq!(embeddings.0.lock().unwrap()[2], ["alfa", "zeta"]);
    let searched = state
        .qdrant
        .lock()
        .unwrap()
        .iter()
        .filter_map(|request| {
            if request.using.as_deref() != Some("keyword") {
                return None;
            }
            let query::Variant::Nearest(vector) =
                request.query.as_ref().unwrap().variant.as_ref().unwrap()
            else {
                panic!("Expected a nearest-neighbor query");
            };
            let vector_input::Variant::Dense(vector) = vector.variant.as_ref().unwrap() else {
                panic!("Expected dense embeddings");
            };
            Some(vector.data.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(searched, [[1.0, 0.0], [0.0, 1.0], [1.0, 0.0], [0.0, 1.0]]);
    let keywords = local
        .iter()
        .find(|body| body["model"] == "keywords")
        .unwrap();
    assert_eq!(keywords["format"]["required"], json!(["keywords"]));
    assert_eq!(
        keywords["format"]["properties"]["keywords"]["items"]["type"],
        "string"
    );
    assert!(
        keywords
            .get("tools")
            .is_none_or(|tools| tools == &json!([]))
    );
    assert!(messages(keywords)[0].1.contains(INPUT));
    for request in local.iter().filter(|body| body["model"] == "response") {
        assert_eq!(request["format"]["required"], json!(["response"]));
        assert_eq!(
            request["format"]["properties"]["response"]["type"],
            "string"
        );
    }
    let analysis = local
        .iter()
        .find(|body| body["model"] == "analysis")
        .unwrap();
    let scores = &analysis["format"]["properties"]["scores"];
    assert_eq!(scores["required"], json!([mistake.id]));
    assert_eq!(scores["additionalProperties"], false);

    let hosted = config(&url, "anthropic");
    let anthropic = |name| {
        CompletionAgentFactory::build(&hosted.resolve_channel(name).unwrap(), Some("fixture"))
            .unwrap()
    };
    service.response_agent = anthropic("response");
    service.learning_agent = anthropic("learning");
    service.pronunciation_agent = anthropic("pronunciation");
    service.analysis_agent = anthropic("analysis");
    assert_eq!(
        service.generate_response(&params).await.0.unwrap().response,
        REPLY
    );
    assert_eq!(
        service
            .generate_analysis(analyze())
            .await
            .0
            .unwrap()
            .mistake_scores[&mistake.id]
            .score,
        1
    );
    let hosted = state.llm.lock().unwrap();
    // Before routing moved into the integration, Anthropic inserted the assistant
    // prefill after the feature's history, immediately before its final user prompt.
    for model in ["response", "learning", "pronunciation", "analysis"] {
        let local_requests = local
            .iter()
            .filter(|body| body["model"] == model)
            .collect::<Vec<_>>();
        let hosted_requests = hosted
            .iter()
            .filter(|body| body["model"] == model)
            .collect::<Vec<_>>();
        assert_eq!(local_requests.len(), hosted_requests.len());
        for (local, hosted) in local_requests.into_iter().zip(hosted_requests) {
            let mut local = messages(local);
            let hosted_messages = messages(hosted);
            let ("system", preamble) = local.remove(0) else {
                panic!("Missing system instruction");
            };
            let system = &hosted["system"];
            assert_eq!(
                system.as_str().or_else(|| system[0]["text"].as_str()),
                Some(preamble)
            );
            if model == "response" {
                assert_eq!(local.len(), 4);
                assert!(local[0].1.contains(CORPUS_EXAMPLE));
                assert_eq!(
                    local[1..],
                    [
                        ("user", "Hola"),
                        ("assistant", "¿Qué necesitas?"),
                        ("user", INPUT)
                    ]
                );
            } else {
                assert_eq!(local.len(), 1);
                assert_eq!(local[0].0, "user");
            }
            local.insert(local.len() - 1, ("assistant", "{"));
            assert_eq!(hosted_messages, local, "{model} message order");
        }
    }
    assert!(
        local
            .iter()
            .filter(|body| body["model"] == "response")
            .nth(1)
            .unwrap()
            .to_string()
            .contains("PREVIOUS OUTPUT FAILED")
    );
}

fn scanned_source() -> PlanSource {
    PlanSource::File {
        content_type: "application/pdf".into(),
        base64: BASE64_STANDARD.encode(SCAN),
    }
}

fn scan_images() -> Vec<Vec<u8>> {
    let parsing::ParsedContent::ScannedImages(images) =
        parsing::extract_clean_text(parsing::DocumentSource::Pdf(SCAN.to_vec())).unwrap()
    else {
        panic!("Expected two scan pages");
    };
    images
}

#[tokio::test]
async fn local_activity_and_scanned_plan_use_their_configured_channels() {
    let state = Arc::new(Requests::default());
    let url = llm_fixture(state.clone()).await;
    let local = config(&url, "ollama");
    let planning_channel = local.resolve_channel("planning").unwrap();
    let planning = CompletionAgentFactory::build(&planning_channel, None).unwrap();
    let ocr =
        CompletionAgentFactory::build_ocr(&local.resolve_channel("ocr").unwrap(), None).unwrap();

    let user = UserState::new(uuid::Uuid::new_v4());
    let mut context = user.build_action_context(vec![]);
    context.dialect = DIALECT;
    let request = GenerateActivityRequest {
        user_id: user.user_id,
        branch_id: user.active_branch_id,
        description: Some("Find the station".into()),
        context,
    };
    let (activity, _) = activity::generate(planning.as_ref(), &request).await;
    let activity = activity.unwrap();
    assert_eq!(activity.title, "Find the station");
    assert_eq!(activity.shared_context, "You are downtown");
    assert_eq!(activity.learner_brief, "Pregunta por la estación");
    assert_eq!(activity.partner_brief, "The station is on the left");
    assert_eq!(
        activity
            .goals
            .iter()
            .map(|goal| (goal.goal.as_str(), goal.dialect))
            .collect::<Vec<_>>(),
        vec![
            ("Pide indicaciones", DIALECT),
            ("Confirma la dirección", DIALECT)
        ]
    );
    let activity_request = state.llm.lock().unwrap().pop().unwrap();
    assert_eq!(activity_request["model"], "planning");
    assert_eq!(activity_request["options"]["temperature"], 0.7);
    assert_eq!(
        activity_request["format"]["properties"]["goals"]["items"]["type"],
        "string"
    );

    let reasoning = config(&format!("{url}/v1"), "openai");
    let unsupported = CompletionAgentFactory::build(
        &reasoning.resolve_channel("planning").unwrap(),
        Some("fixture"),
    )
    .unwrap();
    let (error, usage) = activity::generate(unsupported.as_ref(), &request).await;
    assert!(
        error
            .unwrap_err()
            .to_string()
            .contains("transmit temperature 0.7")
    );
    assert!(usage.is_empty());
    assert!(state.llm.lock().unwrap().is_empty());

    let generator = PlanGenerator::new(planning, ocr);
    let scanned = planning_handler::generate_plan(
        &generator,
        PlanRequest {
            dialect: DIALECT,
            source: scanned_source(),
        },
    )
    .await
    .unwrap();
    assert_eq!(scanned.title, "Scan plan");
    assert_eq!(scanned.steps[0].title, "Vocabulary");
    let plain = planning_handler::generate_plan(
        &generator,
        PlanRequest {
            dialect: DIALECT,
            source: PlanSource::File {
                content_type: "text/plain".into(),
                base64: BASE64_STANDARD.encode("Plain-text import"),
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(plain.title, scanned.title);

    let requests = state.llm.lock().unwrap();
    assert_eq!(requests.len(), 3);
    let [ocr, scanned, plain] = requests.as_slice() else {
        unreachable!()
    };
    assert_eq!(ocr["model"], "vision");
    assert_eq!(ocr["options"]["num_predict"], 4096);
    assert_eq!(
        messages(ocr),
        [(
            "user",
            "Transcribe the text from these document images exactly. Output ONLY the text. Do not add markdown formatting, commentary, or preambles."
        )]
    );
    let encoded_pages = ocr["messages"][0]["images"].as_array().unwrap();
    assert_eq!(encoded_pages.len(), 2);
    let pages = encoded_pages
        .iter()
        .map(|page| BASE64_STANDARD.decode(page.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(pages, scan_images());
    assert_eq!(image::load_from_memory(&pages[0]).unwrap().height(), 72);
    assert_eq!(image::load_from_memory(&pages[1]).unwrap().height(), 84);
    assert_eq!(scanned["model"], "planning");
    assert_eq!(
        scanned["format"]["properties"]["dialect"]["const"],
        DIALECT.id()
    );
    assert!(messages(scanned).last().unwrap().1.contains(TRANSCRIPT));
    assert_eq!(plain["model"], "planning");
    assert!(
        messages(plain)
            .last()
            .unwrap()
            .1
            .contains("Plain-text import")
    );
}

#[tokio::test]
async fn ocr_client_has_a_sixty_second_timeout() {
    check_ocr_timeout(None).await;
}

#[tokio::test]
async fn ocr_client_uses_the_configured_channel_timeout() {
    check_ocr_timeout(Some(300)).await;
}

async fn check_ocr_timeout(configured_seconds: Option<u64>) {
    let (received, notification) = tokio::sync::oneshot::channel();
    let received = Arc::new(Mutex::new(Some(received)));
    let url = serve(Router::new().route(
        "/api/chat",
        post(move || {
            let received = received.clone();
            async move {
                received.lock().unwrap().take().unwrap().send(()).unwrap();
                std::future::pending::<()>().await;
            }
        }),
    ))
    .await;
    let model_config = match configured_seconds {
        Some(seconds) => serde_yaml::from_str(&format!(
            "
default_provider: fixture
providers:
  fixture: {{ driver: ollama, base_url: {url}, model: vision }}
channels:
  ocr: {{ request_timeout_seconds: {seconds} }}
"
        ))
        .unwrap(),
        None => config(&url, "ollama"),
    };
    let channel = model_config.resolve_channel("ocr").unwrap();
    let agent = CompletionAgentFactory::build_ocr(&channel, None).unwrap();
    let deadline = Duration::from_secs(configured_seconds.unwrap_or(60));
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let transcription =
        tokio::spawn(
            async move { OcrService::transcribe_images(&[vec![1]], agent.as_ref()).await },
        );
    notification.await.unwrap();
    tokio::time::sleep_until(started + deadline - Duration::from_secs(1)).await;
    assert!(!transcription.is_finished());
    let error = tokio::time::timeout_at(
        started + deadline + Duration::from_millis(50),
        transcription,
    )
    .await
    .expect("the OCR client should time out at the channel deadline")
    .unwrap()
    .unwrap_err();
    assert!(started.elapsed() >= deadline);
    let diagnostic = format!("{error:#}");
    assert!(
        diagnostic.to_lowercase().contains("timed out"),
        "{diagnostic}"
    );
}

#[tokio::test]
#[ignore = "requires downloaded Qwen text and vision models on an Ollama server"]
async fn live_ollama_keywords_and_ocr() {
    let url = std::env::var("OLLAMA_TEST_URL").unwrap();
    let vision_model =
        std::env::var("OLLAMA_TEST_VISION_MODEL").unwrap_or_else(|_| "qwen3.5:0.8b".into());
    let models: LlmConfig = serde_yaml::from_str(&format!(
        "
default_provider: local
providers:
  local: {{ driver: ollama, base_url: {url}, model: 'qwen3:0.6b' }}
channels:
  ocr: {{ model: '{vision_model}' }}
"
    ))
    .unwrap();
    let keyword_agent =
        CompletionAgentFactory::build(&models.resolve_channel("keywords").unwrap(), None).unwrap();
    let keywords = KeywordExtractor::new(keyword_agent)
        .extract(INPUT, DIALECT)
        .await
        .unwrap();
    assert!(!keywords.is_empty());
    assert!(keywords.windows(2).all(|pair| pair[0] <= pair[1]));
    println!("Live keyword extraction returned {keywords:?}");

    let ocr =
        CompletionAgentFactory::build_ocr(&models.resolve_channel("ocr").unwrap(), None).unwrap();
    let transcript = OcrService::transcribe_images(&scan_images(), ocr.as_ref())
        .await
        .unwrap();
    assert!(transcript.contains(TRANSCRIPT), "{transcript}");
    println!("Live OCR returned {transcript:?}");
}
