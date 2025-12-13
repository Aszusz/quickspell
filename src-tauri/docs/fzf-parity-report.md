# fzf Feature Parity Report

Analysis of fzf source (`pattern.go`, `options.go`, `result.go`, `result_x86.go`) vs nucleo-matcher to identify gaps for wrapper implementation.

## Executive Summary

nucleo provides **scoring parity** with fzf but lacks:

1. Tiebreaker system (critical)
2. Extended search syntax
3. Scheme presets
4. Some term types

## 1. Scoring Algorithm

| Feature           | fzf          | nucleo                | Gap  |
| ----------------- | ------------ | --------------------- | ---- |
| Fuzzy matching    | V1, V2       | V2 equivalent         | None |
| Exact substring   | Yes          | `AtomKind::Substring` | None |
| Score calculation | Same formula | Same formula          | None |

**No wrapper needed** - nucleo's scoring matches fzf.

## 2. Case Matching

| Mode            | fzf                  | nucleo                  | Gap  |
| --------------- | -------------------- | ----------------------- | ---- |
| Smart (default) | `CaseSmart`          | `CaseMatching::Smart`   | None |
| Ignore          | `CaseIgnore` / `-i`  | `CaseMatching::Ignore`  | None |
| Respect         | `CaseRespect` / `+i` | `CaseMatching::Respect` | None |

**fzf implementation:**

```go
caseSensitive := caseMode == CaseRespect ||
    caseMode == CaseSmart && text != lowerText
```

**No wrapper needed** - nucleo has full parity.

## 3. Normalization

| Feature               | fzf                    | nucleo                 | Gap  |
| --------------------- | ---------------------- | ---------------------- | ---- |
| Unicode normalization | `--literal` to disable | `Normalization::Smart` | None |
| Smart normalization   | Yes                    | Yes                    | None |

**No wrapper needed**.

## 4. Tiebreakers (CRITICAL GAP)

fzf has a full tiebreaker system. nucleo returns only score.

### 4.1 Available Criteria

| Criterion    | fzf | nucleo | Description                     |
| ------------ | --- | ------ | ------------------------------- |
| `byScore`    | Yes | Yes    | Match quality (primary)         |
| `byLength`   | Yes | **No** | Shorter haystack wins           |
| `byPathname` | Yes | **No** | Match closer to filename wins   |
| `byBegin`    | Yes | **No** | Match closer to start wins      |
| `byEnd`      | Yes | **No** | Match closer to end wins        |
| `byChunk`    | Yes | **No** | Shorter matched span wins       |
| `byIndex`    | Yes | **No** | Original order (implicit final) |

### 4.2 Scheme Presets

```go
// options.go - parseScheme()
"default"  -> [byScore, byLength]
"path"     -> [byScore, byPathname, byLength]
"history"  -> [byScore]
```

### 4.3 fzf Rank Structure

```go
// result.go
type Result struct {
    item   *Item
    points [4]uint16  // 4 tiebreaker slots
}
```

Each criterion fills a slot in `points[]`. Lower values = better rank.

### 4.4 fzf Score Inversion

```go
// result.go - buildResult()
case byScore:
    val = math.MaxUint16 - util.AsUint16(score)
```

Scores inverted so ascending sort works (lower = better).

### 4.5 fzf Pathname Calculation

```go
// result.go - buildResult()
case byPathname:
    // Find last path delimiter
    for idx := int(item.text.Length) - 1; idx >= 0; idx-- {
        if item.text.Get(idx) == '/' || item.text.Get(idx) == '\\' {
            lastDelim = idx + 1
            break
        }
    }
    // Distance from delimiter to match start
    if lastDelim <= minBegin {
        val = util.AsUint16(minBegin - lastDelim)
    }
```

Match in filename portion (after last `/`) gets lower value = higher rank.

### 4.6 fzf Fast Comparison (x86)

```go
// result_x86.go
func compareRanks(irank Result, jrank Result, tac bool) bool {
    left := *(*uint64)(unsafe.Pointer(&irank.points[0]))
    right := *(*uint64)(unsafe.Pointer(&jrank.points[0]))
    if left < right {
        return true
    } else if left > right {
        return false
    }
    return (irank.item.Index() <= jrank.item.Index()) != tac
}
```

Casts `[4]uint16` to single `uint64` for one-instruction comparison.

### 4.7 Wrapper Implementation Required

```rust
// Proposed Rank struct
struct Rank {
    points: u64,  // packed [score_inv:16][pathname:16][length:16][unused:16]
    index: u32,
}

impl Rank {
    fn new(score: u32, pathname_dist: u16, length: u16, index: usize) -> Self {
        let score_inv = (u16::MAX as u32).saturating_sub(score.min(u16::MAX as u32)) as u16;
        let points = ((score_inv as u64) << 48)
                   | ((pathname_dist as u64) << 32)
                   | ((length as u64) << 16);
        Self { points, index: index as u32 }
    }
}
```

## 5. Extended Search Syntax (MODERATE GAP)

fzf supports complex query expressions:

| Syntax       | Meaning           | nucleo                      |
| ------------ | ----------------- | --------------------------- |
| `foo`        | Fuzzy match       | Yes (default)               |
| `'foo`       | Exact match       | Yes (`AtomKind::Substring`) |
| `^foo`       | Prefix match      | Yes (`AtomKind::Prefix`)    |
| `foo$`       | Suffix match      | Yes (`AtomKind::Suffix`)    |
| `!foo`       | Inverse (exclude) | **No**                      |
| `foo \| bar` | OR condition      | **No**                      |
| `^foo$`      | Exact equal       | Yes (`AtomKind::Exact`)     |

### 5.1 fzf Term Types

```go
// pattern.go
const (
    termFuzzy         // default fuzzy
    termExact         // 'quoted or from --exact
    termExactBoundary // unused?
    termPrefix        // ^prefix
    termSuffix        // suffix$
    termEqual         // ^exact$
)
```

### 5.2 Wrapper Implementation

For full parity, parse query string and:

1. Split on `|` for OR groups
2. Check `!` prefix for negation
3. Check `'`, `^`, `$` for term type
4. Run multiple patterns, combine results

**Priority: Medium** - Most users don't use extended syntax.

## 6. Additional Options

| Option        | fzf                 | nucleo            | Priority |
| ------------- | ------------------- | ----------------- | -------- |
| `--nth=N`     | Match on Nth field  | Manual in wrapper | Medium   |
| `--delimiter` | Field separator     | Manual split      | Medium   |
| `--tac`       | Reverse input order | Sort flag         | Low      |
| `--track`     | Track current item  | UI concern        | N/A      |

## 7. Implementation Roadmap

### Phase 1: Tiebreakers (HIGH)

- [ ] Implement `Rank` struct with u64 packing
- [ ] Add `byLength` tiebreaker
- [ ] Add `byPathname` for path scheme
- [ ] Add `byIndex` as final tiebreaker

### Phase 2: Schemes (HIGH)

- [ ] Implement scheme presets (default, path, history)
- [ ] Map `SearchScheme::Path` to `[score, pathname, length]`

### Phase 3: Extended Syntax (MEDIUM)

- [ ] Parse `!` negation
- [ ] Parse `|` OR groups
- [ ] Combine multi-pattern results

### Phase 4: Field Matching (LOW)

- [ ] `--nth` equivalent via config
- [ ] Custom delimiters

## 8. Current fuzzy.rs Status

```rust
// Current implementation
struct Rank {
    points: u64,  // [score_inv:16][length:16][unused:32]
    index: u32,
}
```

**Implements:** `[byScore, byLength, byIndex]` = fzf default scheme

**Missing for path scheme:** `byPathname` calculation

## References

- [fzf/src/pattern.go](https://github.com/junegunn/fzf/blob/master/src/pattern.go)
- [fzf/src/options.go](https://github.com/junegunn/fzf/blob/master/src/options.go)
- [fzf/src/result.go](https://github.com/junegunn/fzf/blob/master/src/result.go)
- [fzf/src/result_x86.go](https://github.com/junegunn/fzf/blob/master/src/result_x86.go)
- [nucleo-matcher docs](https://docs.rs/nucleo-matcher)
