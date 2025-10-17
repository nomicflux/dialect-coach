# Developer Guide: Understanding the Dialect Coach Codebase

Welcome to Dialect Coach - an AI-powered language learning app that helps users practice authentic dialect variations through conversational AI. This guide will help you understand the codebase structure, key concepts, and how to work with this Rust application.

## Quick Start for Rust Developers

If you're already proficient with Rust and want to jump in quickly:

1. **Architecture**: Multi-crate workspace with WebSocket-based real-time chat
2. **Key Tech**: Axum backend, Yew frontend (WASM), Claude AI, Qdrant vector DB
3. **Main Flow**: User → WebSocket → RAG retrieval → Claude AI → Response

But read on for the full picture...

---

## 📁 Project Structure

This is a **Cargo workspace** with 4 main crates:

```
dialect-coach/
├── shared/             # Common types and business logic
├── backend/            # Axum WebSocket server 
├── frontend/           # Yew WASM web app
└── corpus-processor/   # CLI tool for managing dialect data
```

### Workspace Dependencies

The `Cargo.toml` at the root defines shared dependencies and uses Rust 2024 edition. Each crate has its own `Cargo.toml` but inherits common deps from the workspace.

---

## 🧠 Core Concepts

### 1. **Dialect Learning Through AI**
- Users chat with AI agents that speak in specific dialect variations
- Supported: Spanish (6 dialects), Arabic (5), French (5)  
- AI responses are grounded in authentic dialect corpus examples

### 2. **RAG (Retrieval-Augmented Generation)**
- Vector database (Qdrant) stores ~11K dialect text examples
- Before generating responses, system retrieves relevant examples
- Claude AI mimics patterns from retrieved examples

### 3. **Real-time Communication**
- WebSocket protocol for instant message exchange
- JSON message format with session management
- Frontend reconnects automatically on connection loss

---

## 🏗️ Architecture Deep Dive

### Data Flow Overview

```
User Input → Frontend → WebSocket → Backend → RAG Retrieval → Claude AI → Response
     ↑                                                                          ↓
     └─────────────── WebSocket Response ←─────────────── JSON Message ←───────┘
```

### Message Protocol

All communication uses this JSON structure:
```rust
// shared/src/models/message.rs
pub struct Message {
    pub session_id: Uuid,      // Groups related conversation
    pub participant_id: String, // "user1" or "agent"  
    pub content: String,        // The actual message text
    pub language: String,       // BCP-47 code like "es-MX"
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata, // Teaching mode, formality level
}
```

---

## 📦 Crate-by-Crate Guide

### 1. **shared/** - Common Foundation

**Purpose**: Types and logic used by both frontend and backend

**Key Files**:
```rust
// lib.rs - Main exports
pub mod models;  // Data structures
pub mod logic;   // Business logic  
pub mod tts;     // Text-to-speech traits

// models/dialect.rs - The heart of the system
pub enum Dialect {
    SpanishMexican,      // es-MX
    SpanishArgentinian,  // es-AR
    ArabicEgyptian,      // ar-EG
    // ... 16 total variants
}

impl Dialect {
    pub fn from_bcp47(code: &str) -> Option<Self>  // "es-MX" → SpanishMexican
    pub fn name(&self) -> &'static str            // Human-readable name
    pub fn language(&self) -> Language            // Groups dialects by language
}
```

**Teaching Concepts**:
```rust
pub enum TeachingMode {
    Immersive,    // Just chat naturally
    Corrective,   // Point out errors  
    Explanatory,  // Explain grammar/culture
}

pub enum Formality {
    Formal,       // Polite, proper grammar
    Casual,       // Everyday conversation
    DialectRich,  // Heavy dialect markers
    Slang,        // Informal/street language
}
```

### 2. **backend/** - The Engine

**Purpose**: WebSocket server that orchestrates AI conversations

#### Entry Point (`main.rs`)
```rust
#[tokio::main]  
async fn main() -> Result<()> {
    // 1. Load environment (.env file)
    dotenvy::dotenv().ok();
    
    // 2. Setup logging
    tracing_subscriber::registry().init();
    
    // 3. Initialize services (order matters!)
    let qdrant = QdrantService::from_env().await?;        // Vector DB
    let embeddings = EmbeddingService::new()?;            // Text → vectors  
    let agent = AgentService::from_env(qdrant, embeddings)?; // AI orchestrator
    let tts = AzureTtsProvider::from_env()?;             // Text → speech
    
    // 4. Build Axum router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket::websocket_handler))    // Main chat endpoint
        .nest("/api/tts", tts_router);                     // Speech synthesis
        
    // 5. Start server
    axum::serve(listener, app).await?;
}
```

#### WebSocket Handler (`websocket.rs`)
```rust
pub async fn websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    // Split socket into sender/receiver  
    let (sender, mut receiver) = socket.split();
    
    while let Some(Ok(msg)) = receiver.next().await {
        // 1. Parse JSON message
        let parsed_msg: Message = serde_json::from_str(&msg)?;
        
        // 2. Extract dialect from language code
        let dialect = Dialect::from_bcp47(&parsed_msg.language)?;
        
        // 3. Update conversation history
        let mut histories = state.session_histories.lock().await;
        let history = histories.entry(parsed_msg.session_id).or_default();
        history.push(format!("User: {}", parsed_msg.content));
        
        // 4. Generate AI response
        let response = state.agent.generate_response(
            &parsed_msg.content,
            dialect,
            formality,
            teaching_mode,  
            &history,
        ).await?;
        
        // 5. Send back to client
        let response_msg = Message::new(session_id, "agent", response, language);
        sender.send(serde_json::to_string(&response_msg)?).await?;
    }
}
```

#### AI Agent (`agent_service.rs`) - The Heart of the System

This is where the magic happens:

```rust
impl AgentService {
    pub async fn generate_response(
        &self,
        user_message: &str,
        dialect: Dialect,
        formality: Formality,
        teaching_mode: TeachingMode,
        conversation_history: &[String],
    ) -> Result<String> {
        
        // STEP 1: Enhanced query building
        // Combines recent history with current message for better context
        let query_text = if conversation_history.len() >= 2 {
            let recent_history: Vec<String> = conversation_history
                .iter().rev().take(3).rev().cloned().collect();
            format!("{}\\n\\nCurrent message: {}", 
                   recent_history.join("\\n"), user_message)
        } else {
            user_message.to_string()
        };
        
        // STEP 2: Multi-vector retrieval (3 different embeddings!)
        
        // Content embedding - semantic similarity
        let content_embedding = self.embeddings.embed_text(&query_text).await?;
        
        // Style embedding - captures formality/dialect patterns  
        let style_query = format!("{} response in {}", 
                                 formality.as_str(), dialect.name());
        let style_embedding = self.embeddings.embed_text(&style_query).await?;
        
        // Topic embedding - conversation continuity
        let topic_embedding = if !conversation_history.is_empty() {
            let topic_summary = conversation_history.join(" ");
            Some(self.embeddings.embed_text(&topic_summary).await?)
        } else { None };
        
        // STEP 3: Parallel retrieval from vector database
        let content_examples = self.qdrant
            .search_dialect_examples(content_embedding, dialect, 10).await?;
        let style_examples = self.qdrant  
            .search_dialect_examples(style_embedding, dialect, 10).await?;
        let topic_examples = if let Some(emb) = topic_embedding {
            self.qdrant.search_dialect_examples(emb, dialect, 10).await?
        } else { Vec::new() };
            
        // STEP 4: Add random samples for stylistic variety
        let formality_levels = match formality {
            Formality::Casual => vec![Formality::Casual, Formality::DialectRich],
            Formality::Slang => vec![Formality::Slang, Formality::DialectRich],
            // etc...
        };
        let random_samples = self.qdrant
            .random_dialect_samples(dialect, formality_levels, 15).await?;
            
        // STEP 5: Combine and deduplicate (up to 50 examples!)
        let mut all_examples = Vec::new();
        all_examples.extend(content_examples);
        all_examples.extend(style_examples); 
        all_examples.extend(topic_examples);
        all_examples.extend(random_samples);
        
        let unique_examples: Vec<_> = all_examples.into_iter()
            .collect::<std::collections::HashSet<_>>()  // Dedupe by content
            .into_iter().take(50).collect();
            
        // STEP 6: Build prompt with examples-first approach
        let rag_context = self.format_examples(&unique_examples, formality);
        
        let system_prompt = format!(
            "{}\\n\\n# YOUR ROLE\\n{}\\n\\n# CRITICAL RULES\\n{}",
            rag_context,
            self.role_description(dialect, formality),
            self.teaching_instructions(teaching_mode)
        );
        
        // STEP 7: Generate with Claude
        let agent = self.client.agent(&self.model_name)
            .preamble(&system_prompt)
            .max_tokens(match teaching_mode {
                TeachingMode::Immersive => 128,    // Short and natural
                TeachingMode::Corrective => 256,   // Room for corrections
                TeachingMode::Explanatory => 512,  // Room for explanations
            })
            .temperature(1.1)  // High creativity for natural speech
            .build();
            
        let response = agent.prompt(user_message).await?;
        Ok(response)
    }
}
```

### 3. **frontend/** - The User Interface

**Purpose**: Yew-based WASM app that renders in the browser

#### Architecture
```rust
// main.rs - Entry point
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();  // Mount root component
}

// app.rs - Root component  
#[function_component(App)]
pub fn app() -> Html {
    // State management
    let messages = use_state(|| Vec::<Message>::new());
    let websocket_service = use_state(|| None::<WebSocketService>);
    let tts_service = use_state(|| TtsService::new());
    
    // WebSocket connection management
    let connect_websocket = {
        let websocket_service = websocket_service.clone();
        use_callback(move |_, _| {
            let service = WebSocketService::new("ws://localhost:3000/ws");
            websocket_service.set(Some(service));
        }, ())
    };
    
    html! {
        <div class="app">
            <ChatWindow messages={(*messages).clone()} />
            <InputBox 
                on_send={on_message_send}
                on_speech_toggle={toggle_speech}
            />
            <SpeechControls 
                is_listening={is_listening}
                on_toggle={toggle_speech}
            />
        </div>
    }
}
```

#### Key Frontend Services
```rust
// services/websocket.rs - WebSocket management
pub struct WebSocketService {
    ws: WebSocket,
    connection_state: ConnectionState,
}

impl WebSocketService {
    pub fn new(url: &str) -> Self {
        let ws = WebSocket::new(url).expect("Failed to create WebSocket");
        
        // Setup message handler
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            let data = e.data().as_string().unwrap();
            let message: Message = serde_json::from_str(&data).unwrap();
            // Handle incoming message...
        }) as Box<dyn FnMut(_)>);
        
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        
        Self { ws, connection_state: ConnectionState::Connected }
    }
    
    pub fn send_message(&self, message: &Message) {
        let json = serde_json::to_string(message).unwrap();
        self.ws.send_with_str(&json).unwrap();
    }
}

// services/speech.rs - Web Speech API integration
pub struct SpeechService {
    recognition: SpeechRecognition,
    synthesis: SpeechSynthesis,
}

impl SpeechService {
    pub fn start_listening(&self) -> Result<(), JsValue> {
        self.recognition.start()?;
        Ok(())
    }
    
    pub fn speak(&self, text: &str, language: &str) -> Result<(), JsValue> {
        let utterance = SpeechSynthesisUtterance::new_with_text(text)?;
        utterance.set_lang(language);
        self.synthesis.speak(&utterance);
        Ok(())
    }
}
```

### 4. **corpus-processor/** - Data Management  

**Purpose**: CLI tool for importing and processing dialect text corpora

```rust
// main.rs - CLI interface using clap
#[derive(Parser)]
enum Commands {
    /// Process text files into dialect documents
    Process {
        #[arg(long)]
        input: PathBuf,   // Directory of text files
        #[arg(long)]  
        output: PathBuf,  // Output JSONL file
    },
    
    /// Upload processed documents to Qdrant
    Upload {
        #[arg(long)]
        input: PathBuf,   // JSONL file to upload
    },
    
    /// List supported dialects
    List,
}

async fn main() -> Result<()> {
    match Commands::parse() {
        Commands::Process { input, output } => {
            // 1. Load text files
            let documents = loaders::load_directory(&input)?;
            
            // 2. Chunk into smaller pieces (~512 chars)
            let chunks = chunking::chunk_documents(documents)?;
            
            // 3. Generate embeddings
            let embedded = embeddings::embed_chunks(chunks).await?;
            
            // 4. Save as JSONL
            processor::save_jsonl(&output, embedded)?;
        }
        
        Commands::Upload { input } => {
            // Load and upload to Qdrant vector database
            let documents = processor::load_jsonl(&input)?;
            qdrant::upload_documents(documents).await?;
        }
        
        Commands::List => {
            // Show all supported dialect variants
            for dialect in Dialect::all() {
                println!("{}: {} ({})", 
                        dialect.id(), 
                        dialect.name(),
                        dialect.language().name());
            }
        }
    }
}
```

---

## 🔧 Development Setup

### Prerequisites
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Frontend build tool
cargo install trunk

# Optional: WebSocket testing
cargo install websocat
```

### Environment Variables
Create `.env` file in project root:
```bash
# Required - AI service
ANTHROPIC_API_KEY=your_key_here
ANTHROPIC_MODEL=claude-3-5-sonnet  # optional

# Required - Vector database  
QDRANT_URL=https://your-instance.cloud.qdrant.io:6334
QDRANT_API_KEY=your_qdrant_key

# Required - Text-to-speech
AZURE_SPEECH_KEY=your_azure_key
AZURE_SPEECH_REGION=eastus  # or your region

# Optional - Logging
RUST_LOG=debug
```

### Running the Application

**Backend Server:**
```bash
cd backend
cargo run
# Server starts on http://localhost:3000
# WebSocket endpoint: ws://localhost:3000/ws
# Health check: http://localhost:3000/health
```

**Frontend Development:**
```bash  
cd frontend
trunk serve  
# Dev server on http://localhost:8080
# Auto-reloads on file changes
```

**Corpus Processing:**
```bash
cd corpus-processor

# Process dialect texts
cargo run -- process --input ./corpus_data/spanish_mexican/ --output ./processed/es-mx.jsonl

# Upload to vector database
cargo run -- upload --input ./processed/es-mx.jsonl

# List supported dialects
cargo run -- list
```

### Testing

**Unit Tests:**
```bash
cargo test                           # All crates
cargo test -p dialect-coach-backend  # Backend only
```

**Integration Testing:**
```bash  
# Test WebSocket flow with websocat
websocat ws://localhost:3000/ws

# Send test message (paste this JSON):
{"session_id":"550e8400-e29b-41d4-a716-446655440000","participant_id":"user1","content":"¡Hola! ¿Cómo estás?","language":"es-MX","timestamp":"2025-10-16T18:51:24Z","metadata":{"formality":"Casual","teaching_mode":"Immersive"}}
```

---

## 🎯 Key Learning Paths

### For Rust Web Development
1. **Axum Patterns**: Study `backend/src/main.rs` for router setup
2. **WebSocket Handling**: `backend/src/websocket.rs` shows real-time communication  
3. **State Management**: `AppState` struct demonstrates shared state with `Arc<Mutex<>>`
4. **Error Handling**: Consistent use of `anyhow::Result` throughout

### For AI/RAG Systems
1. **Vector Search**: `backend/src/qdrant_service.rs` - semantic similarity search
2. **Multi-Vector Retrieval**: `agent_service.rs` lines 77-161 - advanced RAG strategy
3. **Prompt Engineering**: `agent_service.rs` lines 196-300 - examples-first prompting
4. **Embedding Generation**: `backend/src/embedding_service.rs` - text vectorization

### For WASM Frontend
1. **Yew Components**: `frontend/src/app.rs` - functional components with hooks
2. **WebSocket Client**: `frontend/src/services/websocket.rs` - browser WebSocket API
3. **Web Speech API**: `frontend/src/services/speech.rs` - speech recognition/synthesis
4. **WASM-JS Interop**: Various `web_sys` and `js_sys` usage patterns

### For Domain Modeling  
1. **Type Safety**: `shared/src/models/` - strongly-typed domain model
2. **Enum Patterns**: `dialect.rs` - complex enums with behavior
3. **Serialization**: Consistent `serde` usage for JSON handling
4. **Protocol Design**: WebSocket message structure

---

## 🔍 Understanding the RAG Strategy

The system uses an advanced multi-vector retrieval approach:

### Why Multiple Embeddings?
```rust
// Single embedding approach (traditional)
let embedding = embed_text(user_message);  
let examples = search_similar(embedding, 5);  // Limited context

// Multi-vector approach (this system) 
let content_emb = embed_text(enhanced_query);     // WHAT they're talking about
let style_emb = embed_text("casual Mexican Spanish"); // HOW to respond  
let topic_emb = embed_text(conversation_history);     // CONTEXT continuity

let examples = search_multiple([content_emb, style_emb, topic_emb], 50);
```

### Retrieval Strategy Benefits
- **Content Embedding**: Finds topically relevant examples
- **Style Embedding**: Finds examples matching desired formality/dialect
- **Topic Embedding**: Maintains conversation flow and context
- **Random Sampling**: Prevents over-fitting to semantic similarity
- **50 Examples**: Rich context for pattern mimicry

---

## 🚀 Extension Points

### Adding New Dialects
1. **Update Enum**: Add variant to `shared/src/models/dialect.rs`
2. **BCP-47 Mapping**: Add language code mapping in `from_bcp47()`
3. **Voice Mapping**: Add TTS voice in `backend/src/tts_service.rs`  
4. **Corpus Data**: Process dialect texts with `corpus-processor`

### Adding New Features
1. **New Endpoints**: Add routes in `backend/src/main.rs`
2. **New Components**: Create in `frontend/src/components/`
3. **New Services**: Add to respective `services/` directory
4. **Shared Types**: Define in `shared/src/models/`

### Different AI Models
1. **Rig Framework**: Supports OpenAI, Anthropic, local models
2. **Replace AgentService**: Swap out Claude implementation
3. **Embedding Models**: Change Fastembed model in `embedding_service.rs`
4. **Vector Database**: Qdrant client is abstracted in `qdrant_service.rs`

---

## 📚 Additional Resources

- **Existing Documentation**: See `ARCHITECTURE.md`, `CODE_STATE_ANALYSIS.md`, `WARP.md` 
- **Tutorials**: Check `tutorials/` directory for dependency-specific guides
- **Examples**: `backend/examples/test_agent.rs` shows agent usage
- **Project Status**: `PROJECT_STATUS.md` tracks current implementation state

## 🎯 Next Steps for New Developers

1. **Start Simple**: Run the backend, test with `websocat`
2. **Understand Message Flow**: Trace a message from WebSocket → RAG → Claude → Response
3. **Experiment with Prompts**: Modify `agent_service.rs` prompt templates
4. **Add a Feature**: Try implementing a new endpoint or frontend component
5. **Read the Code**: The best documentation is the code itself - it's well-commented!

Happy coding! 🦀✨