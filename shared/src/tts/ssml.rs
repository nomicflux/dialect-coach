//! SSML (Speech Synthesis Markup Language) builder for fine-tuned pronunciation control.
//!
//! This module provides a fluent API for constructing SSML markup that can be used with
//! TTS providers like Google Cloud Text-to-Speech to control:
//! - Pauses and breaks
//! - Emphasis and stress
//! - Prosody (rate, pitch, volume)
//! - Phonetic pronunciation

use std::fmt;

/// Builder for constructing SSML markup
#[derive(Debug, Clone, Default)]
pub struct SsmlBuilder {
    elements: Vec<SsmlElement>,
}

impl SsmlBuilder {
    /// Create a new SSML builder
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Add plain text (will be XML-escaped)
    pub fn add_text<S: Into<String>>(mut self, text: S) -> Self {
        self.elements
            .push(SsmlElement::Text(escape_xml(text.into())));
        self
    }

    /// Add a break (pause) in speech
    ///
    /// # Arguments
    /// * `duration` - Duration like "200ms", "1s", "medium", "strong"
    ///
    /// # Example
    /// ```
    /// use dialect_coach_shared::tts::ssml::SsmlBuilder;
    ///
    /// let ssml = SsmlBuilder::new()
    ///     .add_text("Hello")
    ///     .add_break("500ms")
    ///     .add_text("World")
    ///     .build();
    /// ```
    pub fn add_break<S: Into<String>>(mut self, duration: S) -> Self {
        self.elements.push(SsmlElement::Break(duration.into()));
        self
    }

    /// Add emphasized text
    ///
    /// # Arguments
    /// * `text` - Text to emphasize
    /// * `level` - Emphasis level: "strong", "moderate", "reduced", or "none"
    ///
    /// # Example
    /// ```
    /// use dialect_coach_shared::tts::ssml::SsmlBuilder;
    ///
    /// let ssml = SsmlBuilder::new()
    ///     .add_text("This is")
    ///     .add_emphasis("very", "strong")
    ///     .add_text("important")
    ///     .build();
    /// ```
    pub fn add_emphasis<S: Into<String>, L: Into<String>>(mut self, text: S, level: L) -> Self {
        self.elements.push(SsmlElement::Emphasis {
            text: escape_xml(text.into()),
            level: level.into(),
        });
        self
    }

    /// Add text with prosody control (rate, pitch, volume)
    ///
    /// # Arguments
    /// * `text` - Text to modify
    /// * `prosody` - Prosody settings
    ///
    /// # Example
    /// ```
    /// use dialect_coach_shared::tts::ssml::{SsmlBuilder, Prosody};
    ///
    /// let ssml = SsmlBuilder::new()
    ///     .add_prosody(
    ///         "Speak slowly and quietly",
    ///         Prosody::new()
    ///             .rate("slow")
    ///             .volume("soft")
    ///     )
    ///     .build();
    /// ```
    pub fn add_prosody<S: Into<String>>(mut self, text: S, prosody: Prosody) -> Self {
        self.elements.push(SsmlElement::Prosody {
            text: escape_xml(text.into()),
            rate: prosody.rate,
            pitch: prosody.pitch,
            volume: prosody.volume,
        });
        self
    }

    /// Add phonetically pronounced text
    ///
    /// # Arguments
    /// * `text` - Text to display
    /// * `phoneme` - IPA or X-SAMPA phonetic representation
    /// * `alphabet` - Phonetic alphabet: "ipa" or "x-sampa"
    ///
    /// # Example
    /// ```
    /// use dialect_coach_shared::tts::ssml::SsmlBuilder;
    ///
    /// let ssml = SsmlBuilder::new()
    ///     .add_text("The city is pronounced")
    ///     .add_phoneme("Oaxaca", "waˈhaka", "ipa")
    ///     .build();
    /// ```
    pub fn add_phoneme<S: Into<String>, P: Into<String>, A: Into<String>>(
        mut self,
        text: S,
        phoneme: P,
        alphabet: A,
    ) -> Self {
        self.elements.push(SsmlElement::Phoneme {
            text: escape_xml(text.into()),
            phoneme: phoneme.into(),
            alphabet: alphabet.into(),
        });
        self
    }

    /// Add a say-as element for specific interpretation
    ///
    /// # Arguments
    /// * `text` - Text to interpret
    /// * `interpret_as` - How to interpret: "cardinal", "ordinal", "digits", "fraction",
    ///   "unit", "date", "time", "telephone", "address"
    ///
    /// # Example
    /// ```
    /// use dialect_coach_shared::tts::ssml::SsmlBuilder;
    ///
    /// let ssml = SsmlBuilder::new()
    ///     .add_text("Call me at")
    ///     .add_say_as("5551234567", "telephone")
    ///     .build();
    /// ```
    pub fn add_say_as<S: Into<String>, I: Into<String>>(
        mut self,
        text: S,
        interpret_as: I,
    ) -> Self {
        self.elements.push(SsmlElement::SayAs {
            text: escape_xml(text.into()),
            interpret_as: interpret_as.into(),
        });
        self
    }

    /// Add a paragraph break
    pub fn add_paragraph<S: Into<String>>(mut self, text: S) -> Self {
        self.elements
            .push(SsmlElement::Paragraph(escape_xml(text.into())));
        self
    }

    /// Add a sentence break
    pub fn add_sentence<S: Into<String>>(mut self, text: S) -> Self {
        self.elements
            .push(SsmlElement::Sentence(escape_xml(text.into())));
        self
    }

    /// Build the final SSML string wrapped in <speak> tags
    pub fn build(self) -> String {
        if self.elements.is_empty() {
            return String::from("<speak></speak>");
        }

        let mut result = String::from("<speak>");
        for element in self.elements {
            result.push_str(&element.to_string());
        }
        result.push_str("</speak>");
        result
    }

    /// Build without <speak> wrapper (for embedding in larger SSML)
    pub fn build_fragment(self) -> String {
        let mut result = String::new();
        for element in self.elements {
            result.push_str(&element.to_string());
        }
        result
    }
}

/// Prosody settings for controlling speech characteristics
#[derive(Debug, Clone, Default)]
pub struct Prosody {
    pub rate: Option<String>,
    pub pitch: Option<String>,
    pub volume: Option<String>,
}

impl Prosody {
    /// Create new prosody settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set speech rate
    ///
    /// # Arguments
    /// * `rate` - Rate: "x-slow", "slow", "medium", "fast", "x-fast", or percentage like "80%"
    pub fn rate<S: Into<String>>(mut self, rate: S) -> Self {
        self.rate = Some(rate.into());
        self
    }

    /// Set pitch
    ///
    /// # Arguments
    /// * `pitch` - Pitch: "x-low", "low", "medium", "high", "x-high", or relative like "+10%"
    pub fn pitch<S: Into<String>>(mut self, pitch: S) -> Self {
        self.pitch = Some(pitch.into());
        self
    }

    /// Set volume
    ///
    /// # Arguments
    /// * `volume` - Volume: "silent", "x-soft", "soft", "medium", "loud", "x-loud", or dB like "+6dB"
    pub fn volume<S: Into<String>>(mut self, volume: S) -> Self {
        self.volume = Some(volume.into());
        self
    }
}

/// Internal representation of SSML elements
#[derive(Debug, Clone)]
enum SsmlElement {
    Text(String),
    Break(String),
    Emphasis {
        text: String,
        level: String,
    },
    Prosody {
        text: String,
        rate: Option<String>,
        pitch: Option<String>,
        volume: Option<String>,
    },
    Phoneme {
        text: String,
        phoneme: String,
        alphabet: String,
    },
    SayAs {
        text: String,
        interpret_as: String,
    },
    Paragraph(String),
    Sentence(String),
}

impl fmt::Display for SsmlElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "{}", text),
            Self::Break(duration) => write!(f, "<break time=\"{}\"/>", duration),
            Self::Emphasis { text, level } => {
                write!(f, "<emphasis level=\"{}\">{}</emphasis>", level, text)
            }
            Self::Prosody {
                text,
                rate,
                pitch,
                volume,
            } => {
                write!(f, "<prosody")?;
                if let Some(r) = rate {
                    write!(f, " rate=\"{}\"", r)?;
                }
                if let Some(p) = pitch {
                    write!(f, " pitch=\"{}\"", p)?;
                }
                if let Some(v) = volume {
                    write!(f, " volume=\"{}\"", v)?;
                }
                write!(f, ">{}</prosody>", text)
            }
            Self::Phoneme {
                text,
                phoneme,
                alphabet,
            } => {
                write!(
                    f,
                    "<phoneme alphabet=\"{}\" ph=\"{}\">{}</phoneme>",
                    alphabet, phoneme, text
                )
            }
            Self::SayAs { text, interpret_as } => {
                write!(
                    f,
                    "<say-as interpret-as=\"{}\">{}</say-as>",
                    interpret_as, text
                )
            }
            Self::Paragraph(text) => write!(f, "<p>{}</p>", text),
            Self::Sentence(text) => write!(f, "<s>{}</s>", text),
        }
    }
}

/// Escape XML special characters for safe SSML generation
fn escape_xml(text: String) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let ssml = SsmlBuilder::new().add_text("Hello World").build();
        assert_eq!(ssml, "<speak>Hello World</speak>");
    }

    #[test]
    fn test_xml_escaping() {
        let ssml = SsmlBuilder::new()
            .add_text("Test <tag> & \"quotes\"")
            .build();
        assert_eq!(
            ssml,
            "<speak>Test &lt;tag&gt; &amp; &quot;quotes&quot;</speak>"
        );
    }

    #[test]
    fn test_break() {
        let ssml = SsmlBuilder::new()
            .add_text("Hello")
            .add_break("500ms")
            .add_text("World")
            .build();
        assert_eq!(ssml, "<speak>Hello<break time=\"500ms\"/>World</speak>");
    }

    #[test]
    fn test_emphasis() {
        let ssml = SsmlBuilder::new()
            .add_emphasis("very important", "strong")
            .build();
        assert_eq!(
            ssml,
            "<speak><emphasis level=\"strong\">very important</emphasis></speak>"
        );
    }

    #[test]
    fn test_prosody() {
        let ssml = SsmlBuilder::new()
            .add_prosody("Speak slowly", Prosody::new().rate("slow").pitch("low"))
            .build();
        assert_eq!(
            ssml,
            "<speak><prosody rate=\"slow\" pitch=\"low\">Speak slowly</prosody></speak>"
        );
    }

    #[test]
    fn test_phoneme() {
        let ssml = SsmlBuilder::new()
            .add_phoneme("Oaxaca", "waˈhaka", "ipa")
            .build();
        assert_eq!(
            ssml,
            "<speak><phoneme alphabet=\"ipa\" ph=\"waˈhaka\">Oaxaca</phoneme></speak>"
        );
    }

    #[test]
    fn test_say_as() {
        let ssml = SsmlBuilder::new()
            .add_say_as("5551234567", "telephone")
            .build();
        assert_eq!(
            ssml,
            "<speak><say-as interpret-as=\"telephone\">5551234567</say-as></speak>"
        );
    }

    #[test]
    fn test_paragraph() {
        let ssml = SsmlBuilder::new()
            .add_paragraph("First paragraph.")
            .add_paragraph("Second paragraph.")
            .build();
        assert_eq!(
            ssml,
            "<speak><p>First paragraph.</p><p>Second paragraph.</p></speak>"
        );
    }

    #[test]
    fn test_complex_ssml() {
        let ssml = SsmlBuilder::new()
            .add_text("Welcome to")
            .add_emphasis("Mexico", "moderate")
            .add_break("300ms")
            .add_prosody(
                "where the food is amazing",
                Prosody::new().rate("slow").volume("loud"),
            )
            .build();

        assert!(ssml.contains("<emphasis level=\"moderate\">Mexico</emphasis>"));
        assert!(ssml.contains("<break time=\"300ms\"/>"));
        assert!(ssml.contains(
            "<prosody rate=\"slow\" volume=\"loud\">where the food is amazing</prosody>"
        ));
    }

    #[test]
    fn test_build_fragment() {
        let fragment = SsmlBuilder::new()
            .add_text("Hello")
            .add_break("500ms")
            .build_fragment();
        assert_eq!(fragment, "Hello<break time=\"500ms\"/>");
        assert!(!fragment.contains("<speak>"));
    }
}
