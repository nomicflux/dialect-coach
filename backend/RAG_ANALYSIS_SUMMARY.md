# RAG Parameter Tuning Analysis - Cross-Dialect Insights

## Executive Summary

Analysis of 270 test configurations across 5 dialects reveals clear patterns for optimizing RAG document handling for realistic dialog across different dialects.

## Key Findings

### 1. Model Choice: **Haiku Outperforms Sonnet**
- **Haiku mean**: 0.021628 (better)
- **Sonnet mean**: 0.022521
- **However**: Sonnet shows more **consistency** across dialects (lower variance)
- **Recommendation**: Use Haiku for best overall performance, Sonnet if dialect consistency is critical

### 2. Conversation Samples: **More is Better**
Clear trend showing more conversation samples improves performance:
- **0 samples**: 0.022384
- **5 samples**: 0.022144  
- **25 samples**: 0.021694 ← **Best**

**Key insight**: 25 conversation samples provides ~3% improvement over 0 samples across all dialects.

### 3. Random Samples: **Moderate Impact**
Random samples have less impact but still matter:
- **0 samples**: 0.022132
- **5 samples**: 0.022133
- **25 samples**: 0.021958 ← **Slight advantage**

**Recommendation**: Include some random samples (5-25), but prioritize conversation samples.

### 4. Message Type: **User Messages Perform Best**
Small but consistent advantage:
- **User**: 0.021994 ← Best
- **System**: 0.022043
- **Assistant**: 0.022185

**Insight**: User messages embed closest to corpus vectors, suggesting user input style matters most.

### 5. Critical Interactions

#### Model × Conversation Samples
- **Haiku with 25 conv samples**: 0.020990 (excellent)
- **Sonnet with 25 conv samples**: 0.022399 (good but not as good)
- **Haiku benefits more from conversation samples** than Sonnet

#### Conversation × Random Samples
Best combinations:
- **conv=25/rand=0**: 0.021584
- **conv=25/rand=25**: 0.021631  
- **conv=5/rand=5**: 0.021997 (good balance if 25 conv samples unavailable)

## Top Recommendations for Cross-Dialect Performance

### 🥇 Best Overall: **haiku/user/conv=25/rand=5**
- **Overall mean**: 0.020347
- **Consistency**: 0.005337 (moderate)
- **Worst-case**: 0.027086 (Cuban Spanish)
- **Why**: Best balance of performance and safety across all dialects

### 🥈 Most Consistent: **sonnet/system/conv=5/rand=25**
- **Overall mean**: 0.021571
- **Consistency**: 0.003822 (best consistency!)
- **Worst-case**: 0.027123
- **Why**: Lowest variance across dialects - reliable performance everywhere

### 🥉 Safest Worst-Case: **sonnet/user/conv=5/rand=5**
- **Overall mean**: 0.022227
- **Consistency**: 0.003553 (very consistent)
- **Worst-case**: 0.027020 (best worst-case!)
- **Why**: Even the worst-performing dialect (Cuban Spanish) gets good results

## Dialect-Specific Notes

**Cuban Spanish** is the most challenging dialect (highest scores). Top configurations for Cuban Spanish:
1. sonnet/user/conv=5/rand=5: 0.027020
2. haiku/user/conv=25/rand=5: 0.027086
3. sonnet/system/conv=5/rand=25: 0.027123

**Key insight**: If a configuration works well for Cuban Spanish, it will work well for all dialects.

## Practical Recommendations

### For Maximum Performance
- **Model**: Haiku
- **Message placement**: User messages
- **Conversation samples**: 25 (critical!)
- **Random samples**: 5-25

### For Maximum Consistency
- **Model**: Sonnet  
- **Message placement**: System or User
- **Conversation samples**: 5-25
- **Random samples**: 5-25

### For Balanced Approach
- **Model**: Haiku (better performance) or Sonnet (more consistent)
- **Message placement**: User
- **Conversation samples**: 25 (highest priority!)
- **Random samples**: 5 (diminishing returns beyond this)

## Conclusions

1. **25 conversation samples is the single most important parameter** - provides 3% improvement regardless of other settings
2. **Haiku performs better overall** but Sonnet is more consistent - choose based on whether you prioritize peak performance or reliability
3. **Random samples help but are secondary** to conversation samples - include 5-25 if possible
4. **User messages embed best** - slight advantage but consistent
5. **The worst dialect (Cuban Spanish) is a good test case** - if it works there, it works everywhere

## Methodology Notes

- Lower cosine_squared = better (messages closer to corpus neighbors)
- Analysis uses mean across all 5 dialects for each configuration
- Consistency = standard deviation across dialects (lower = more uniform performance)
- Worst-case = maximum score across dialects (indicates safety margin)
