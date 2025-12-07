# Quick Reference - Aggregation + TopK Debugging

## Run the Test

```bash
# Basic run with INFO logging (recommended)
./run_agg_topk_debug.sh

# With DEBUG logging (more detail)
./run_agg_topk_debug.sh debug

# Focused on aggregation and TopK only
./run_agg_topk_debug.sh focused
```

## Log Prefixes Cheat Sheet

| Prefix | Component | What It Shows |
|--------|-----------|---------------|
| `[AGG-PARTIAL]` | Aggregation | Partial mode operations |
| `[AGG-FINAL]` | Aggregation | Final mode operations |
| `[AGG-BATCH]` | Aggregation | Group creation |
| `[AGG-OUTPUT]` | Aggregation | Output production |
| `[AGG-EARLY-EMIT]` | Aggregation | Memory pressure emissions |
| `[MEMORY]` | Aggregation | Memory before/after emit |
| `[TOPK-INSERT]` | TopK | Batch insertion |
| `[TOPK-COMPACT]` | TopK | Heap compaction |
| `[TOPK-EMIT]` | TopK | Final emission |
| `[TOPK-EARLY]` | TopK | Early termination |

## Key Symbols

- `▼` - Receiving data (input)
- `✓` - Operation complete (success)
- `⚠` - Important event (warning/notice)
- `Δ` - Delta/change in value

## Quick Filters

```bash
# See only aggregation
grep '\[AGG-'

# See only TopK
grep '\[TOPK-'

# See only memory operations
grep '\[MEMORY\]'

# See batch flow between operators
grep -E '(Returning batch to upstream|Receiving batch from)'

# See EmitTo calls
grep 'emit_to='
```

## Typical Flow

```
1. [AGG-*] Received input batch
2. [AGG-BATCH] Created N new groups
3. [AGG-*] After aggregation: total_groups=X
4. [MEMORY] BEFORE emit
5. [MEMORY] AFTER emit (memory freed)
6. [AGG-OUTPUT] Returning batch to upstream
7. [TOPK-INSERT] Receiving batch from aggregation
8. [TOPK-INSERT] Batch processed
9. [TOPK-EMIT] Final emission
```

## Common Issues

### High Memory
Look for: Large `total_groups`, `[AGG-EARLY-EMIT]` messages

### Slow Performance
Look for: High `replacements_total`, frequent `[TOPK-COMPACT]`

### Wrong Results
Look for: Group count mismatches, unexpected `emit_to` values

## Files to Read

1. **LOGGING_GUIDE.md** - Complete log reference
2. **AGGREGATION_TOPK_DEBUG_GUIDE.md** - How to run and debug
3. **CUSTOM_INSTRUMENTATION_EXAMPLE.md** - Add your own logs

## Modified Source Files

- `datafusion/physical-plan/src/aggregates/row_hash.rs` - Aggregation logging
- `datafusion/physical-plan/src/topk/mod.rs` - TopK logging

## Reverting Changes

To remove the logging:
```bash
git checkout datafusion/physical-plan/src/aggregates/row_hash.rs
git checkout datafusion/physical-plan/src/topk/mod.rs
```

## Example Output

```
[AGG-FINAL] Received input batch: rows=8192, current_groups=0
[AGG-BATCH] Created 1523 new groups (total now: 1523)
[AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10)
[MEMORY] BEFORE emit(All): total=289KB, num_groups=10
[MEMORY] AFTER emit(All): total=4KB, emitted_rows=10, memory_freed=285KB
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=10
[TOPK-INSERT] ▼ Receiving batch from aggregation: rows=10, current_heap=0/10
[TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

## Need Help?

1. Check **LOGGING_GUIDE.md** for detailed explanations
2. Run with `./run_agg_topk_debug.sh debug` for more detail
3. Use grep filters to focus on specific components
4. Add custom logging (see CUSTOM_INSTRUMENTATION_EXAMPLE.md)
