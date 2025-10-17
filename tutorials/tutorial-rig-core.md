# Rig Tutorial: LLM Agent Framework

## Overview

**rig-core** is a Rust library for building LLM-powered applications. It provides abstractions for working with various LLM providers (OpenAI, Anthropic, Cohere, etc.) and building agent systems with tool use, RAG (Retrieval-Augmented Generation), and multi-agent workflows.

**Version used in dialect-coach:** `0.8`

**Why we use it:** Provides a unified, type-safe interface for Claude API, making it easy to build AI agents with system prompts, conversation history, and parameters.

## Dependencies

```toml
[dependencies]
rig-core = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Core Concepts

### 1. Client Initialization

Connect to an LLM provider:

```rust
use rig::providers::anthropic::{ClientBuilder, CLAUDE_3_5_SONNET};

let client = ClientBuilder::new(api_key)
    .anthropic_version("2023-06-01")
    .build();
```

## What is RAG?

**RAG (Retrieval-Augmented Generation)** is a technique to improve LLM responses by providing relevant context from external sources.

**The problem:** LLMs are trained on static data and can:
- Hallucinate facts
- Lack specific domain knowledge
- Have outdated information

**The solution:** Before generating a response:
1. **Retrieve** relevant information from your data
2. **Augment** the prompt with that information
3. **Generate** a response based on both the query and retrieved context

**Visual flow:**
```
User Query: "What's the Spanish slang for 'cool'?"
      ↓
  [Embed query into vector]
      ↓
  [Search vector database]
      ↓
Retrieved Examples:
- "Está bien chido ese carro" (Mexican slang)
- "Qué guay tu camisa" (Spanish slang)
      ↓
  [Inject into LLM prompt]
      ↓
LLM System Prompt:
"You are a Spanish tutor. Use these authentic examples:
 1. 'Está bien chido ese carro'
 2. 'Qué guay tu camisa'

 Respond to: What's the Spanish slang for 'cool'?"
      ↓
LLM Response: "In Mexican Spanish, 'chido' means cool.
In Spain Spanish, 'guay' means cool..."
```

**Why it works:** The LLM can see actual examples and base its answer on real data, not just training memory.

**RAG vs Fine-tuning:**
- **RAG:** Add data dynamically at query time (flexible, no retraining)
- **Fine-tuning:** Retrain model on your data (expensive, static)

### 2. Agents

Agents are LLM instances with configuration:

```rust
let agent = client
    .agent(CLAUDE_3_5_SONNET)
    .preamble("You are a helpful assistant")
    .temperature(0.7)
    .max_tokens(1024)
    .build();
```

## Agent Configuration Parameters

Understanding what each parameter does:

**`preamble` (System Prompt):**
- **What:** Instructions that set the agent's behavior
- **When:** Always included at the start of every conversation
- **Use:** Define role, personality, constraints, output format
- **Example:** `"You are a helpful coding assistant. Provide concise answers with code examples."`

**`temperature` (0.0 - 1.0):**
- **What:** Controls randomness in responses
- **Low (0.0-0.3):** Deterministic, factual, focused
  - Use for: Code generation, factual Q&A, data extraction
- **Medium (0.4-0.7):** Balanced creativity and coherence
  - Use for: General conversation, explanations
- **High (0.8-1.0):** Creative, diverse, unpredictable
  - Use for: Creative writing, brainstorming
- **Example:** `temperature(0.1)` for consistent SQL generation

**`max_tokens`:**
- **What:** Maximum length of response (in tokens, ~4 chars each)
- **Why limit:** Cost control, prevent rambling, ensure concise responses
- **Typical values:**
  - 256: Short answers
  - 1024: Standard responses (dialect-coach uses this)
  - 4096: Long-form content
- **Example:** `max_tokens(512)` for brief explanations

**Complete configuration example:**
```rust
let agent = client
    .agent(CLAUDE_3_5_SONNET)
    .preamble(
        "You are a dialect coach. Provide authentic examples from \
         the target dialect. Keep responses conversational and educational."
    )
    .temperature(0.3)  // Factual but slightly varied
    .max_tokens(1024)  // Allow detailed explanations
    .build();
```

### 3. Prompts

Send messages to agents:

```rust
let response = agent
    .prompt("What is the capital of France?")
    .await?;

println!("Response: {}", response);
```

### 4. Conversation Context

Maintain multi-turn conversations:

```rust
let mut history = Vec::new();

history.push(("user", "Hello!"));
let response1 = agent.prompt_with_history("Hello!", &history).await?;
history.push(("assistant", &response1));

history.push(("user", "How are you?"));
let response2 = agent.prompt_with_history("How are you?", &history).await?;
```

## Step-by-Step: Building a CLI Chatbot

### Step 1: Basic Completion

```rust
use rig::providers::anthropic::{ClientBuilder, CLAUDE_3_5_SONNET};
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;

    let client = ClientBuilder::new(&api_key)
        .anthropic_version("2023-06-01")
        .build();

    let agent = client
        .agent(CLAUDE_3_5_SONNET)
        .build();

    let response = agent
        .prompt("Explain quantum computing in one sentence")
        .await?;

    println!("Claude: {}", response);

    Ok(())
}
```

### Step 2: Agent with System Prompt

```rust
let agent = client
    .agent(CLAUDE_3_5_SONNET)
    .preamble(
        "You are a helpful coding assistant. \
         Provide concise, accurate answers with code examples when relevant."
    )
    .max_tokens(2048)
    .build();

let response = agent
    .prompt("How do I reverse a string in Rust?")
    .await?;

println!("{}", response);
```

### Step 3: Interactive Chat Loop

```rust
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;

    let client = ClientBuilder::new(&api_key)
        .anthropic_version("2023-06-01")
        .build();

    let agent = client
        .agent(CLAUDE_3_5_SONNET)
        .preamble("You are a friendly conversational assistant.")
        .build();

    let mut history: Vec<String> = Vec::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() || input == "exit" {
            break;
        }

        // Add to history
        history.push(format!("User: {}", input));

        // Build context from history
        let context = if history.len() > 10 {
            history.iter().rev().take(10).rev().cloned().collect::<Vec<_>>().join("\n")
        } else {
            history.join("\n")
        };

        // Create prompt with context
        let full_prompt = if context.is_empty() {
            input.to_string()
        } else {
            format!("Previous conversation:\n{}\n\nUser: {}", context, input)
        };

        match agent.prompt(&full_prompt).await {
            Ok(response) => {
                println!("Claude: {}", response);
                history.push(format!("Assistant: {}", response));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
```

### Step 4: Structured Output

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Analysis {
    sentiment: String,
    confidence: f32,
    summary: String,
}

async fn analyze_text(agent: &Agent, text: &str) -> Result<Analysis, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Analyze the following text and respond ONLY with valid JSON matching this structure: \
         {{\"sentiment\": \"positive|negative|neutral\", \"confidence\": 0.0-1.0, \"summary\": \"brief summary\"}}\n\n\
         Text: {}",
        text
    );

    let response = agent.prompt(&prompt).await?;

    // Extract JSON from response
    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];

    let analysis: Analysis = serde_json::from_str(json_str)?;

    Ok(analysis)
}
```

## How dialect-coach Uses rig-core

### 1. Client Initialization (agent_service.rs:20-37)

Setting up Anthropic client:

```rust
pub fn from_env(qdrant: Arc<QdrantService>, embeddings: Arc<EmbeddingService>) -> Result<Self> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;
    let model_name = std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| CLAUDE_3_5_SONNET.to_string());

    let client = ClientBuilder::new(&api_key)
        .anthropic_version("2023-06-01")
        .build();

    tracing::info!("Initialized Anthropic client with model: {}", model_name);

    Ok(Self {
        client,
        model_name,
        qdrant,
        embeddings,
    })
}
```

**Pattern:** Load API key from environment, configure client version, store for reuse.

### 2. Building Agent with System Prompt (agent_service.rs:260-286)

Creating agents with rich context:

```rust
// Build system prompt with RAG context
let system_content = format!(
    "{}\n\n\
    # YOUR ROLE\n\
    {}. Your responses must sound EXACTLY like the authentic examples above.\n\n\
    # CRITICAL RULES\n\
    1. MIMIC THE PATTERNS: Study the examples and copy their vocabulary, grammar\n\
    2. MAINTAIN FORMALITY: Match the {} level\n\
    {}\n\
    4. BE BRIEF: Keep responses conversational\n\
    {}\n\n\
    Now respond naturally as a local {} speaker would.",
    rag_context,
    role_desc,
    formality_label,
    teaching_rules,
    history_context,
    dialect.name()
);

// Create agent
let agent = self
    .client
    .agent(&self.model_name)
    .preamble(&system_content)
    .max_tokens(1024)
    .build();
```

**Pattern:** Build dynamic system prompts with RAG examples and instructions. The preamble sets the agent's behavior.

### 3. Generating Responses (agent_service.rs:289-292)

Simple prompt execution:

```rust
let response = agent
    .prompt(user_message)
    .await
    .context("Failed to get completion from Claude")?;

tracing::info!(
    "Generated response for dialect {} ({} chars)",
    dialect.name(),
    response.len()
);
```

**Pattern:** Use `.prompt()` for single-turn interactions. Error handling with context for debugging.

### 4. RAG Integration (agent_service.rs:40-228)

Combining retrieval with generation:

```rust
pub async fn generate_response(
    &self,
    user_message: &str,
    dialect: Dialect,
    formality: Formality,
    teaching_mode: TeachingMode,
    conversation_history: &[String],
) -> Result<String> {
    // Step 1: Generate embeddings for retrieval
    let content_embedding = self
        .embeddings
        .embed_text(&query_text)
        .context("Failed to generate embedding")?;

    // Step 2: Retrieve relevant examples from vector DB
    let examples = self
        .qdrant
        .search_dialect_examples(content_embedding, dialect, 10)
        .await
        .context("Failed to search examples")?;

    // Step 3: Build RAG context from retrieved examples
    let mut rag_context = format!("# AUTHENTIC {} EXAMPLES\n\n", dialect.name());
    for (i, doc) in examples.iter().enumerate() {
        rag_context.push_str(&format!("{}. \"{}\"\n", i + 1, doc.content));
    }

    // Step 4: Create agent with RAG context in system prompt
    let system_prompt = format!("{}\n\nRespond using patterns from examples.", rag_context);

    let agent = self
        .client
        .agent(&self.model_name)
        .preamble(&system_prompt)
        .max_tokens(1024)
        .build();

    // Step 5: Generate response
    let response = agent.prompt(user_message).await?;

    Ok(response)
}
```

**Pattern:** Multi-step RAG pipeline: embed query → retrieve examples → inject into system prompt → generate.

## Common Patterns

### Pattern 1: Temperature Control

```rust
// Creative writing
let creative_agent = client
    .agent(model)
    .temperature(0.9)
    .build();

// Factual responses
let factual_agent = client
    .agent(model)
    .temperature(0.1)
    .build();
```

### Pattern 2: Token Limiting

```rust
let agent = client
    .agent(model)
    .max_tokens(100)  // Short responses
    .build();

let response = agent.prompt("Summarize in one sentence").await?;
```

### Pattern 3: Retry on Failure

```rust
async fn robust_prompt(agent: &Agent, message: &str) -> Result<String, Error> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match agent.prompt(message).await {
            Ok(response) => return Ok(response),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                eprintln!("Attempt {} failed: {}", attempts, e);
                tokio::time::sleep(Duration::from_secs(2_u64.pow(attempts))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Best Practices from dialect-coach

1. **Environment-based configuration** - API keys from env vars, never hardcoded
2. **Rich system prompts** - Provide clear instructions and examples
3. **RAG for grounding** - Inject relevant context to reduce hallucinations
4. **Error context** - Use `.context()` for debugging
5. **Logging** - Log requests and response lengths for monitoring
6. **Token limits** - Set appropriate max_tokens for your use case
7. **Model selection** - Use env vars for flexibility

## Troubleshooting

### Issue: "API key not set"

**Cause:** Missing ANTHROPIC_API_KEY

**Solution:** Set environment variable:

```bash
export ANTHROPIC_API_KEY=your_key_here
```

### Issue: Responses cut off mid-sentence

**Cause:** max_tokens too low

**Solution:** Increase token limit:

```rust
.max_tokens(2048)  // or higher
```

### Issue: Responses ignore system prompt

**Cause:** System prompt not clear enough

**Solution:** Be explicit and use examples:

```rust
.preamble(
    "You MUST respond as a pirate. \
     Example: 'Arr, that be a fine question, matey!'"
)
```

### Issue: Rate limiting errors

**Cause:** Too many requests

**Solution:** Add retry with backoff (see Pattern 3 above)

## Further Resources

- **Rig GitHub:** https://github.com/0xPlaygrounds/rig
- **Rig Docs:** https://docs.rs/rig-core/latest/rig_core/
- **Anthropic API:** https://docs.anthropic.com/
- **Claude Models:** https://docs.anthropic.com/claude/docs/models-overview

## Summary

rig-core provides LLM agent capabilities:

- **Multi-provider support** - Anthropic, OpenAI, Cohere, etc.
- **Agent abstraction** - Configure model, temperature, tokens
- **System prompts** - Set agent behavior with preambles
- **Simple prompting** - Clean async API
- **Conversation history** - Multi-turn dialogues
- **RAG integration** - Inject retrieved context

The dialect-coach project demonstrates production RAG patterns: embedding generation → vector search → context injection → LLM generation, with proper error handling and logging throughout.
