# Dialect IPA Pronunciation Guide for TTS

Purpose: Document the specific IPA sound changes for each dialect (relative to a standard/baseline) so an LLM can generate `pronunciation_text` that a TTS engine reads with dialect-appropriate pronunciation.

Scope: Dialects with TTS voices currently configured.

---

## Spanish Dialects

**Baseline:** Standard Latin American Spanish (5 vowels /a e i o u/, seseo, yeismo, full consonant retention)


### Mexican Spanish (ElevenLabs)

#### Consonants
- /s/ fully retained in ALL positions — syllable-final, word-final, pre-consonantal
  - This is a key distinguishing feature: Mexican Spanish is an "s-retaining" dialect
- /x/ realized as velar fricative [x] (strong, not glottal [h])
  - e.g., "mejor" = [meˈxoɾ], "gente" = [ˈxente]
- /tʃ/ retained as full affricate [tʃ]
  - Northern Mexico may deaffricate to [ʃ], but central Mexico (Mexico City) retains [tʃ]
- Trill /r/ → voiced retroflex fricative [ʐ] (assibilated r)
  - Common in Mexico City, especially among middle-class female speakers
  - Also appears word-initially and after /n l/: "rojo" = [ˈʐoxo]
- Voiceless plosives have very short VOT: /p/ ~15ms, /t/ ~20ms, /k/ ~49ms
- Voiced plosives have long negative VOT (true voicing, not just unaspirated): /b/ ~−65ms, /d/ ~−84ms, /ɡ/ ~−73ms
- /tl/ cluster retained as single syllable onset — unique to Mexican Spanish from Nahuatl substrate:
  - e.g., "Tlaxcala" = [tlaʃˈkala], "atlas" = [ˈatlas], "tlapalería" = [tlapaleˈɾia]
  - Other Spanish dialects would syllabify /t.l/ across syllable boundary
- Word-final /n/ remains alveolar [n] (NOT velarized [ŋ] as in Caribbean dialects)
  - e.g., "pan" = [pan], "están" = [esˈtan]
- Intervocalic /d/ weakening in -ado present but less extreme than Caribbean:
  - "cansado" → [kanˈsaðo] ~ [kanˈsao] (deletion less common than in Cuban)

#### Vowels
- Unstressed vowel reduction — distinctive feature of central Mexican Spanish:
  - Vowels shorten, centralize, devoice, or fully elide, especially in contact with /s/
  - e.g., "trastes" /ˈtɾastes/ → [ˈtɾasts], "pues" /pwes/ → [ps]
  - "noche" /ˈnotʃe/ → [ˈnotʃ], "leche" /ˈletʃe/ → [ˈletʃ]
  - Most frequent between voiceless consonants and word-finally after voiceless consonants
- Vowel devoicing most common in these environments:
  - Between /s/ and voiceless stop: "especial" → [espe̥ˈsjal]
  - Word-final after voiceless consonant: "noche" → [ˈnotʃe̥] ~ [ˈnotʃ]
  - In rapid speech, entire unstressed syllables can disappear: "necesito" → [nesˈsito] ~ [nsˈito]
- Highland Mexican Spanish (Mexico City, Puebla, Guanajuato) has MORE vowel reduction than lowland (Veracruz, coastal)
- Vowels in contact with /s/ are most vulnerable to devoicing/elision

#### Prosody
- Syllable-timed rhythm
- Final rising intonation on declaratives common in some regions (especially questions formed without inversion)
- Relatively clear, measured articulation compared to Caribbean varieties
- Diminutive suffix -ito/-ita very productive and shifts stress: "momento" → "momentito" [momenˈtito]
  - More frequent in Mexican Spanish than in most other dialects; affects rhythm

### Argentinian Spanish — Rioplatense (ElevenLabs + Azure)

#### Consonants
- Sheismo (sheísmo): /ʝ/ → [ʃ] (voiceless postalveolar fricative)
  - Dominant in Buenos Aires among younger speakers (sound change now complete)
  - ⟨ll⟩ and ⟨y⟩ both = [ʃ]: "calle" = [ˈkaʃe], "yo" = [ʃo], "lluvia" = [ˈʃuβ̞ja]
  - Older speakers and western regions: [ʒ] (voiced, = zheísmo)
  - This is THE most distinctive Rioplatense feature
- /s/ → [h] (aspiration) before consonants
  - e.g., "esto" /ˈesto/ → [ˈehto], "dos tres" → [ˈdoh ˈtɾeh]
  - Before vowels and utterance-finally: [s] retained in educated speech
  - Lower sociolects: aspiration extends to all coda positions, culminating in deletion
- Word-final /ɾ/ frequently deleted in rapid informal speech
  - e.g., "comer" /koˈmeɾ/ → [koˈme]
- /x/ realized as velar [x] (not glottal)
- Intervocalic /d/ deletion in -ado: "cansado" → [kanˈsao]
- /tʃ/ retained as full affricate [tʃ] (no deaffrication)
- Word-final /n/ may velarize to [ŋ] in some speakers but less pervasive than Caribbean
- /ɾ/ and /r/ distinction preserved; /ɾ/ may delete word-finally in casual speech (see above)
- Frequent /ɾ/ deletion in rapid informal speech simplifies codas → favours CV rhythmic pattern
  - e.g., "hablar" → [aˈβ̞la], "salir" → [saˈli]

#### Vowels
- Standard 5-vowel system, fully preserved
- No vowel reduction (unlike Mexican Spanish)
- No notable allophonic variation

#### Prosody
- Italian-substrate intonation: distinctive rising-falling pitch contours
  - "Long fall" pattern: early peak alignment on pre-nuclear pitch accents
  - Yes/no questions often use falling bitonal boundary tone (opposite of most Spanish dialects which rise)
  - Broad focus declaratives: final fall begins earlier than in other Spanish dialects
- Perceived "sing-song" or "Italian-like" melodic quality
- Syllable-timed but with rhythmic influence from Italian
- Voseo affects phonological shape of verb forms — changes stress position in some conjugations:
  - Tú tienes [ˈtjenes] → Vos tenés [teˈneh] (stress shifts to final syllable, /s/ aspirated)
  - Tú puedes [ˈpweðes] → Vos podés [poˈðeh]
  - This stress shift is systematic and affects TTS rhythm

### Cuban Spanish (ElevenLabs + Azure)

#### Consonants
- /s/ weakening — pervasive across ALL sociolects:
  - Syllable-final /s/ → [h] (aspiration) or ∅ (full deletion)
  - e.g., "estos" /ˈestos/ → [ˈehtoh] or [ˈetoh]
  - Pre-consonantal: aspiration dominant: "mosca" → [ˈmohka]
  - Word-final before pause: deletion dominant: "tres" → [tɾe]
  - Progression: [s] → [h] → ∅ (complete continuum in conversational speech)
- Lambdacism: syllable-final /ɾ/ → [l] (r → l)
  - e.g., "puerto" /ˈpweɾto/ → [ˈpwelto], "amor" /aˈmoɾ/ → [aˈmol]
  - "carne" /ˈkaɾne/ → [ˈkalne]
  - More frequent in eastern and central Cuba
- Liquid assimilation and gemination (western Cuba):
  - Syllable-final /l/ and /ɾ/ assimilate to following consonant → geminates
  - e.g., "pulpo" /ˈpulpo/ → [ˈpuppo], "caldo" /ˈkaldo/ → [ˈkaddo]
- /x/ → [h] (glottal fricative, weak)
  - e.g., "gente" = [ˈhente], "mejor" = [meˈhoɾ]
- Final /n/ → [ŋ] (velarization)
  - e.g., "pan" /pan/ → [paŋ], "hablan" /ˈablan/ → [ˈaβ̞laŋ]
- Intervocalic /d/ deletion:
  - -ado → [-ao] or [-au̯]: "cansado" → [kanˈsao]
  - -ido → [-io]: "comido" → [koˈmio]
  - Also in other intervocalic positions: "nada" → [ˈnaa] or [ˈna]
- /tʃ/ may weaken to [ʃ] in some speakers (deaffrication)
- /ɾ/ may also become [j] (glide) in some environments: "comer" → [koˈmej]
- Hypercorrection: some speakers reverse lambdacism and produce /l/ → [ɾ], creating doubly confused liquids
- Social variation in Miami Cuban Spanish: some speakers reverse /s/ weakening as a marker of prestige/education
  - Deliberate retention of [s] to differentiate from speakers associated with /s/ lenition
- Word-final /ɾ/ may also be fully deleted: "hablar" → [aˈβ̞la]

#### Vowels
- Vowels tend to be more open and longer to compensate for consonant weakening
- Compensatory lengthening when following consonant is deleted:
  - When /s/ deletes before consonant, preceding vowel may lengthen: "estos" [ˈeːtoh]
  - When /ɾ/ or /l/ assimilates, preceding vowel may also lengthen
- Anticipatory vowel nasalization when /n/ follows (spreading before the velarized [ŋ]):
  - "pan" → [pãŋ], "un" → [ũŋ]

#### Prosody
- Syllable-timed with Caribbean rhythmic patterns
- Tendency toward CV syllable structure (due to coda consonant weakening)
  - Coda simplification creates more open syllables: CVCC → CVC → CV
  - This gives Cuban Spanish its characteristic "flowing" quality
- Rapid speech rate
- Rising intonation on declaratives
- Phrase-final lengthening of vowels
- Rhythmic patterns closer to Caribbean English creoles than to highland Spanish

### Colombian Spanish — Bogota (ElevenLabs + Azure)

#### Consonants
- Conservative variety — consonants generally fully articulated
- /s/ fully retained in ALL positions (no aspiration, no deletion)
  - Like Mexican Spanish, this is a defining "highland conservative" feature
- /x/ → [h] (glottal fricative) — "in all regions of Colombia"
  - e.g., "gente" = [ˈhente], "mejor" = [meˈhoɾ]
  - Lighter/weaker than Mexican velar [x]
- /b d ɡ/ after ANY consonant → full plosives [b d ɡ]
  - NOT just after nasals/laterals as in most dialects
  - e.g., "pardo" = [ˈpaɾdo] (full [d]), "barba" = [ˈbaɾba] (full [b]), "algo" = [ˈalɡo], "peligro" = [peˈliɡɾo], "desde" = [ˈdezde]
  - This is distinctive: most dialects would use approximants in some of these positions
- Intervocalic /b d ɡ/ → approximants [β̞ ð̞ ɣ˕] and may be elided
  - "Bogota" may be pronounced [bo.oˈta] (with /ɡ/ elided)
- Yeismo: /ʎ/ → /ʝ/ (complete in younger speakers)
  - Some older Andean speakers retain /ʎ/ distinction (increasingly rare)
- Intervocalic /d/ deletion in -ado occurs but less frequently than in Caribbean dialects
- Word-final /ɾ/ realization depends on following context:
  - Before a vowel-initial word: tap [ɾ], approximant [ɹ], or lateral [l]
  - Before a consonant or pause: any of tap [ɾ], trill [r], lateral [l], or elided ∅
  - e.g., "comer algo" → [koˈmeɾ ˈalɣ˕o] or [koˈmel ˈalɣ˕o]
- Word-final /n/ remains alveolar [n] (NOT velarized [ŋ] like Caribbean): "pan" = [pan]

#### Vowels
- Clean 5-vowel system, fully preserved
- No vowel reduction
- No diphthongization of monophthongs
- Hiatus maintained more in formal speech (less gliding than Caribbean varieties)
  - "mi amigo" more likely to preserve hiatus [mi aˈmiɣ˕o] than to glide [mjaˈmiɣ˕o]
- Clear, crisp vowel articulation — considered one of the "clearest" Spanish dialects
- No anticipatory nasalization of vowels before nasal consonants (unlike Caribbean)

#### Prosody
- Clear, measured articulation
- Even syllable timing
- Moderate speech rate
- Widely considered prestigious for clarity and completeness of articulation
- Bogota intonation patterns are often used as a model for "neutral" Latin American Spanish in media
- Declaratives have gentle falling contour on final stressed syllable
- Questions with ¿ marker show moderate final rise (less dramatic than Caribbean)

---

## French Dialects

**Baseline:** Standard Metropolitan French (Parisian): 12-15 oral vowels, 3-4 nasal vowels, uvular /ʁ/, no affrication, phrase-final stress


### Quebec French (ElevenLabs + Azure)

#### Consonants
- Affrication of /t/ and /d/ before high front vowels and glides:
  - /t/ → [t͡s] before /i y ɥ j/: "tu" /ty/ → [t͡sy], "petit" → [pət͡si], "tiens" → [t͡sjɛ̃]
  - /d/ → [d͡z] before /i y ɥ j/: "dis" /di/ → [d͡zi], "du" /dy/ → [d͡zy], "dieu" → [d͡zjø]
  - In rapid/casual speech, the stop may fully assimilate: /ty/ → [sy], /di/ → [zi]
  - This is the single most salient Quebec French consonant feature
- /ʁ/ (uvular fricative) — same as Metropolitan French in most positions
  - May be devoiced to [χ] word-finally: "pour" → [puχ]
  - Up to 9 variants documented: uvular trill [ʀ], fricative [ʁ], voiceless [χ], approximant [ɹ], vocalized [ɚ], retroflex [ɻ], and occasionally deletion
  - Weaker variants in coda and intervocalic onset positions
- Final obstruent-liquid cluster simplification:
  - Word-final /bɾ bɫ tɾ dɾ/ → liquid drops: "table" /tabl/ → [tab], "quatre" /katʁ/ → [kat], "trouble" /tʁubl/ → [tʁub]

#### Vowels
- Laxing of high vowels in closed syllables (short, NOT before /ʁ ʒ z v/):
  - /i/ → [ɪ]: "vite" /vit/ → [vɪt], "site" → [sɪt]
  - /y/ → [ʏ]: "juste" /ʒyst/ → [ʒʏst], "lune" → [lʏn]
  - /u/ → [ʊ]: "tout" /tut/ → [tʊt], "poule" → [pʊl]
  - Always in stressed syllables; sometimes absent in unstressed syllables
- Diphthongization of long vowels in closed syllables:
  - /ɛː/ → [aɪ̯]: "fete" → [faɪ̯t], "tete" → [taɪ̯t]
  - /øː/ → [øʏ̯]: "meule" → [møʏ̯l]
  - /oː/ → [oʊ̯]: "cote" → [koʊ̯t]
  - /ɑː/ → [ɑʊ̯]: "pate" → [pɑʊ̯t]; before /ʁ/: [ɑɔ̯] "rare" → [ʁɑɔ̯ʁ]
- Nasal vowels — very different from modern Parisian:
  - /ɛ̃/ → [ẽɪ̯̃] ~ [æ̃ɪ̯̃] (always diphthongized): "vin" → [vẽɪ̯̃]
  - /ɔ̃/ → [ɒ̃ʊ̯̃] (always diphthongized): "bon" → [bɒ̃ʊ̯̃]
  - /ɑ̃/ → [ã] ~ [æ̃]: "dans" → [dãs]
  - /œ̃/ → [œ̃ʏ̯̃] ~ [ɚ̃] ~ [ʌ̃ɹ]: "un" → [œ̃ʏ̯̃]
- /a/ vs /ɑ/ distinction preserved (merged in modern Metropolitan French)
  - "patte" [pat] vs "pate" [pɑːt]
- /ɛ/ vs /ɛː/ vowel length distinction preserved
- Schwa elision — common contractions:
  - "je suis" → "chu" [ʃy]
  - "je ne sais pas" → "ché pas" [ʃepa]

#### Prosody
- More initial-syllable stress than Metropolitan French (which is strictly phrase-final)
- Distinctive intonation contours, perceived as more "punchy"
- Vowel-lengthening as emphasis marker

### African French — West African (ElevenLabs only)

#### Consonants
- /ʁ/ → [r] (alveolar trill) or [ɾ] (alveolar tap)
  - NOT the uvular [ʁ] of Metropolitan French — this is the most salient difference
  - Reflects pre-20th-century French pronunciation and substrate language influence
  - Some speakers use [ɣ] (voiced velar fricative) as alternative
- /ʒ/ and /ʃ/ distinction may be weakened:
  - Some speakers have difficulty fully producing both as distinct phonemes
  - /ʒ/ may be realized as [dʒ] (affricated) under substrate influence
- /t d l n/ may have slightly retroflex or dental articulation depending on substrate language
- Generally fewer consonant cluster reductions than Quebec French

#### Vowels
- Nasal vowel simplification/denasalization:
  - /ɑ̃/ → [a] (especially word-initially): "en" → [a], "ensemble" → [asɑ̃bl]
  - Other nasal vowels may partially denasalize
- Front rounded vowel derounding — major feature:
  - /y/ → [i]: "tu" /ty/ → [ti], "vu" /vy/ → [vi]
  - /ø/ → [e]: "deux" /dø/ → [de], "peu" /pø/ → [pe]
  - /œ/ → [ɛ]: "peur" /pœʁ/ → [pɛr], "heure" → [ɛr]
  - Degree varies by speaker education, substrate language, and formality
- Mid-vowel height contrasts may neutralize:
  - /e/ vs /ɛ/ → often merged to [e] or [ɛ]
  - /o/ vs /ɔ/ → often merged to [o] or [ɔ]
- Overall: fewer vowel contrasts than Metropolitan French (substrate languages typically have 5-7 vowel system vs French 12-15)

#### Prosody
- Syllable-timed rhythm (more regular than Metropolitan French, which is mora-/phrase-timed)
- Substrate-influenced rhythmic patterns from tonal West African languages
  - More even stress distribution across syllables
  - May carry over tonal patterns from L1
- Perceived as more "staccato" or evenly paced

---

## Arabic Dialects

**Baseline:** Modern Standard Arabic (MSA) — 28 consonant phonemes, 6 vowel phonemes /a aː i iː u uː/, pharyngeals /ħ ʕ/, emphatics /tˤ dˤ sˤ ðˤ/, uvular /q/, interdentals /θ ð ðˤ/

### Egyptian Arabic (ElevenLabs + Azure)

#### Consonants
- /q/ (qaf ق) → [ʔ] (glottal stop) — Cairene
  - e.g., MSA /qalb/ → Egyptian [ʔalb] ("heart"), MSA /qamar/ → [ʔamar] ("moon")
  - Upper Egyptian (Sa'idi) retains [ɡ] for /q/: /qalb/ → [ɡalb]
- /dʒ/ (jim ج) → [ɡ] (voiced velar stop) — THE most distinctive Egyptian feature
  - e.g., MSA /dʒabal/ → [ˈɡæbæl] ("mountain"), MSA /dʒamiːl/ → [ɡaˈmiːl] ("beautiful")
  - Even in reciting MSA, Egyptian speakers typically use [ɡ]
- /θ/ (tha ث) → [s] (dental fricative → alveolar)
  - e.g., MSA /θalaːθa/ → [salaːsa] ("three"), MSA /θawb/ → [soːb] ("garment")
- /ð/ (dhal ذ) → [z]
  - e.g., MSA /ðahab/ → [zahab] ("gold"), MSA /ðaːlika/ → [zaːlik] ("that")
- /ðˤ/ (dha ظ) → [zˤ] (emphatic z)
  - e.g., MSA /ðˤuhr/ → [zˤuhr] ("noon")
- Emphatic consonants /tˤ dˤ sˤ zˤ/ preserved; trigger emphasis spreading:
  - Emphasis spreads bidirectionally through the phonological word
  - Affects all vowels in the word: /a/ → [ɑ] throughout
  - Spreads across morphological prefixes, suffixes, and clitics
  - Consonants that trigger: /tˤ dˤ sˤ zˤ/, uvular /q/ (even as [ʔ]), and some instances of /r/
- /ħ ʕ/ (pharyngeals) fully preserved
- Word-final /d/ or /dˤ/ may be devoiced

#### Vowels
- Short /a/ has allophonic split — nearly completely predictable:
  - [æ] in non-emphatic environments: "katab" → [kætæb] ("he wrote")
  - [ɑ] near emphatic consonants: "tabx" → [tˤɑbx] ("cooking")
  - Emphasis spreading turns ALL /a/ in word to [ɑ]
- Long /aː/ same split: [æː] vs [ɑː]
- MSA diphthongs monophthongized:
  - /aj/ → [eː]: MSA /bajt/ → [beːt] ("house"), MSA /ʕajn/ → [ʕeːn] ("eye")
  - /aw/ → [oː]: MSA /yawm/ → [joːm] ("day"), MSA /lawn/ → [loːn] ("color")
- 5 long vowels: /aː iː uː eː oː/ (eː and oː from monophthongization)
- 3 short vowels: /a i u/

#### Stress
- Predictable stress based on syllable weight:
  - Superheavy final syllable (CVːC or CVCC) → final stress
  - Otherwise heavy penult (CVː or CVC) → penult stress
  - Otherwise → antepenult stress
- Not indicated in phonemic transcription because fully predictable

### Levantine Arabic (ElevenLabs + Azure)

#### Consonants
- /q/ (qaf ق) → [ʔ] (glottal stop) — urban varieties (Damascus, Beirut, Amman, Jerusalem)
  - Druze communities retain uvular [q]
  - Rural and Bedouin varieties may use [ɡ]
  - Some Jordanian speakers use [ɡ] or retain [q] depending on word
- /dʒ/ (jim ج) → [ʒ] (voiced postalveolar fricative)
  - e.g., MSA /dʒamiːl/ → [ʒamiːl] ("beautiful"), MSA /dʒaːr/ → [ʒaːr] ("neighbor")
  - Like French ⟨j⟩ or English "pleasure"
- /θ/ (tha ث) → [t] or [s] (varies by word and sub-dialect)
  - e.g., MSA /θalaːθa/ → [tlaːte] or [tlaːti] ("three")
- /ð/ (dhal ذ) → [d] or [z]
  - e.g., MSA /haːða/ → [haːda] ("this")
- /ðˤ/ (dha ظ) → [dˤ] or [zˤ]
- Hamza (ʔ) weakening:
  - Syllable-final /ʔ/ → ∅ with compensatory vowel lengthening: MSA /raʔs/ → [raːs] ("head")
  - /ʔ/ before /i/ → [j]: MSA /saʔil/ → [sajil]
  - Word-initial /ʔ/ often dropped
- /ħ ʕ/ (pharyngeals) fully preserved
- Emphatic consonants preserved with emphasis spreading (similar to Egyptian)

#### Vowels
- 5-vowel system: /a e i o u/ — all can be short or long
  - More vowels than MSA (MSA has 3 short + 3 long; Levantine has 5 short + 5 long)
- MSA diphthongs monophthongized:
  - /aj/ → [eː]: MSA /bajt/ → [beːt] ("house")
  - /aw/ → [oː]: MSA /yawm/ → [joːm] ("day")
- Imala (vowel fronting): /aː/ → [eː] in front phonemic environments
  - e.g., "kitab" (MSA /kitaːb/) → [ktaːb] or [kteːb] ("book")
  - Particularly strong in Lebanese and Palestinian varieties
- Tafkhim (vowel backing): /aː/ → [oː] in back/emphatic environments
  - Triggered by emphatic and back consonants
- Word-final imala with ta marbuta (ة):
  - Final /a/ raised to [æ], [ɛ], [e], or even [i] depending on sub-dialect
  - e.g., MSA /madraːsa/ → Levantine [madrase] or [madrasi] ("school")
- Short vowel reduction: unstressed short vowels may centralize or elide
  - e.g., MSA /kitaːb/ → [ktaːb] (initial /i/ elided)

#### Syllable structure
- Permits far more consonant clusters than MSA:
  - Word-initial CC-: [ktaːb] ("book"), [tneːn] ("two")
  - Word-medial -CCC-: allowed in some environments
  - Epenthetic vowels inserted when clusters become too complex

#### Stress
- Superheavy final → final stress
- Heavy penult → penult stress
- Otherwise → antepenult
- Prefixes and hamzat al-wasl ignored for stress

### Gulf Arabic (Azure only)

#### Consonants
- /q/ (qaf ق) → [ɡ] (voiced velar stop) — traditional/colloquial
  - e.g., MSA /qaːl/ → [ɡaːl] ("he said"), MSA /qalb/ → [ɡalb] ("heart")
  - MSA influence reintroducing [q] in formal registers; free variation [q]~[ɡ] for many speakers
- /dʒ/ (jim ج) → [j] (palatal approximant, like English "y") — Emirati and some Gulf varieties
  - e.g., MSA /dʒadiːd/ → [jadiːd] ("new")
  - Other Gulf sub-dialects retain [dʒ]
- /k/ palatalization → [tʃ] before front vowels /i iː eː/ (unless following consonant is emphatic):
  - e.g., "kalib" → [tʃalib], /kiːf/ → [tʃiːf] ("how")
  - Free variation [kʲ]~[tʃ]
  - Called "kashkasha"
- /ɡ/ palatalization → [dʒ] before front vowels:
  - e.g., /ɡidir/ → [dʒidir] ("he was able")
  - Free variation [ɡʲ]~[dʒ]
- /θ ð ðˤ/ generally RETAINED (more conservative than Egyptian/Levantine)
  - e.g., "thalatha" = [θalaːθa] ("three"), "dhahab" = [ðahab] ("gold")
  - Some younger urban speakers shifting toward [t d dˤ] under Egyptian/Levantine influence
- Emphatic consonants /tˤ dˤ sˤ ðˤ/ preserved with emphasis spreading:
  - Spreading triggers vowel backing: [a] → [ɑ] → [ɒ]
- Additional phoneme: emphatic /pˤ/ exists as a marginal phoneme (no meaningful contrast but appears in certain environments)

#### Vowels
- Short vowel reduction — significant feature:
  - /a/ → [æ] (front, default); → [ɑ] near dorsals/pharyngeals; → [ɒ] near emphatics
  - /i/ and /u/ may merge in unstressed positions: "backness is not phonemically contrastive in short vowels"
  - All short vowels may reduce to [ə] in unstressed positions
  - e.g., /bujuːt/ → [bəjuːt] or [bjuːt] ("houses")
  - "gultulak" ("I told you") → [ɡəltələk]
- Long vowels: /aː iː uː eː oː/
- MSA diphthongs monophthongized (same as other dialects):
  - /aj/ → [eː], /aw/ → [oː]

---

## Japanese Dialects

**Baseline:** Standard Japanese (Tokyo/NHK standard) — mora-timed, 5 vowels /a i ɯ e o/, pitch accent with downstep only


### Tokyo Japanese (ElevenLabs)

This IS the baseline. Included for reference.

#### Consonant allophony (baseline reference)
- /h/ → [ç] before /i/ ("hi" ひ = [çi]), → [ɸ] before /u/ ("fu" ふ = [ɸɯ])
- /s/ → [ɕ] before /i/ ("shi" し = [ɕi])
- /t/ → [tɕ] before /i/ ("chi" ち = [tɕi]), → [ts] before /u/ ("tsu" つ = [tsɯ])
- /z/ → [dʑ] word-initially or after /N/ before /i/ ("ji" じ = [dʑi]), → [ʑ] intervocalically
- /n/ → [ɲ] before /i/ ("ni" に = [ɲi])
- Moraic nasal /N/ (ん) assimilates place:
  - [m] before labials: "shinbun" → [ɕimbɯɴ]
  - [n] before alveolars: "kondo" → [kondo]
  - [ŋ] before velars: "tenki" → [teŋki]
  - [ɴ] word-finally or before pause
  - Nasalized vowel before vowels
- Geminate (long) consonants contrastive: /kite/ ("wearing") vs /kitte/ ("stamp")
- Voiced stops /ɡ/ → [ŋ] (velar nasal) medially in some Tokyo speech (declining feature)

#### Vowels
- High vowel devoicing — MAJOR Tokyo feature (distinguishes Tokyo from most other Japanese dialects):
  - /i/ and /ɯ/ devoice between voiceless consonants: "suki" → [sɯ̥ki], "kita" → [kɪ̥ta]
  - Also word-finally after voiceless consonant: "desu" → [desɯ̥] or [des], "masu" → [masɯ̥]
  - Inhibited if: second consonant is /h/; both consonants are fricatives/affricates; or adjacent mora also has devoiceable vowel

#### Pitch accent
- Tokyo-type: words distinguished ONLY by location of downstep (pitch fall from H to L)
- For N-mora words: N+1 possible accent patterns (N locations for downstep + unaccented)
- Rules:
  - First and second morae always differ in pitch (if first is H, second is L and vice versa)
  - After downstep, all remaining morae are L
  - Unaccented words: LH...H (low start, then high until end of phrase)
- Examples for 2-mora words (4 patterns): HLL, LHL, LHH, LH(L at particle)
- Downsteps CANNOT occur on special morae (ん, long vowel second half, っ)

### Kansai Japanese (ElevenLabs)

#### Consonants
- Geminate → long vowel substitution in verb inflection (MAJOR feature):
  - Past tense -った → -うた:
    - "itta" 言った → "yuːta" 言うた ("said"): /itta/ → /juːta/
    - "katta" 買った → "koːta" 買うた ("bought"): /katta/ → /koːta/
    - "totta" 取った → "toːta" 取うた ("took"): /totta/ → /toːta/
    - "nonda" 飲んだ → "noːda" 飲うだ ("drank"): /nonda/ → /noːda/
  - -te form similarly affected: "itte" → "yuːte", "katte" → "koːte"
- /dzi/ and /dzɯ/ → [zi] and [zɯ] (less affrication than Tokyo)
- /z d r/ confusion in rural areas:
  - e.g., "zenzen" → [denden] ("not at all"), "karada" → [kadara] or [karara] ("body")
- /h/ sometimes replaces /s/ in some speakers
- /m/ sometimes replaces /b/ in some speakers

#### Vowels
- Vowel devoicing RARE — key difference from Tokyo:
  - Tokyo devoices /i ɯ/ between voiceless consonants; Kansai preserves full voicing
  - Tokyo [des] ("desu") vs Kansai [desɯ] (full vowel)
  - Tokyo [sɯ̥ki] vs Kansai [sɯki] ("like")
  - This gives Kansai a "vowel-prominent" sound vs Tokyo's "consonant-prominent" quality
- Monomoraic noun lengthening:
  - /ki/ → [kiː] ("tree"), /me/ → [meː] ("eye"), /to/ → [toː] ("door"), /hi/ → [çiː] ("fire")
  - Makes 1-mora nouns occupy 2 morae
- Standard long vowels may be SHORTENED in 3+ mora words (opposite tendency from above)
- Palatalization of vowels/semivowels after /i/ and /e/ (produces more guttural quality in some environments)

#### Pitch accent — MAJOR difference from Tokyo
- Two-register system: words classified into H-group (高起式 kōki-shiki) and L-group (低起式 teiki-shiki):
  - H-group: first mora starts HIGH: H-L, H-L-L, H-H-L, etc.
  - L-group: first mora starts LOW: L-H, L-H-L, L-L-H, etc.
  - Tokyo only distinguishes by downstep location; Kansai distinguishes by BOTH initial register AND downstep
  - Result: more total pitch patterns than Tokyo → perceived as more "melodic"
- Downstep CAN occur on special morae (nasals ん, long vowels, geminate っ):
  - This is IMPOSSIBLE in Tokyo Japanese
  - e.g., Tokyo [ka.waꜜ] ("river") vs Kansai [kaꜜ.wa] — accent on opposite mora
- Pitch accent differences affect many common words:
  - "hashi" (箸 "chopsticks"): Tokyo LHL, Kansai HLL
  - "hashi" (橋 "bridge"): Tokyo LHH, Kansai LHL
  - "ame" (雨 "rain"): Tokyo LH, Kansai HL
  - "ame" (飴 "candy"): Tokyo HL, Kansai LH
- These pitch differences are the most phonologically significant difference from Tokyo and critically affect TTS naturalness
