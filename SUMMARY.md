# Summary: Aggregation + TopK Debugging Setup

## What Was Created

I've created a comprehensive debugging setup to help you understand how DataFusion's `GroupedHashAggregateStream` and `TopK` operators interact when executing queries with `GROUP BY`, `ORDER BY`, and `LIMIT`.

## Quick Start

```bash
# 1. Run the test
./run_agg_topk_debug.sh

# 2. Read the output and look for log prefixes like:
#    [AGG-*] - Aggregation operations
#    [TOPK-*] - TopK operations
#    [MEMORY] - Memory tracking

# 3. For more detail, see LOGGING_GUIDE.md
```

## What Was Modified

### Source Code Changes (with extensive logging added)

1. **`datafusion/physical-plan/src/aggregates/row_hash.rs`**
   - Added logging for batch reception (`[AGG-PARTIAL]`, `[AGG-FINAL]`)
   - Added logging for group creation (`[AGG-BATCH]`)
   - Added logging for EmitTo calls and emissions (`[MEMORY]`)
   - Added logging for output production (`[AGG-OUTPUT]`)
   - Added logging for early emission (`[AGG-EARLY-EMIT]`)
   - Added logging for blocked groups optimization (`[BLOCKED-GROUPS]`)

2. **`datafusion/physical-plan/src/topk/mod.rs`**
   - Added logging for batch insertion (`[TOPK-INSERT]`)
   - Added logging for heap operations (add/reject rows)
   - Added logging for compaction (`[TOPK-COMPACT]`)
   - Added logging for early termination (`[TOPK-EARLY]`)
   - Added logging for final emission (`[TOPK-EMIT]`)
   - Added logging for memory tracking in RecordBatchStore

### New Files Created

1. **Test File**
   - `datafusion/core/tests/aggregation_topk_debug.rs` - The actual test

2. **Helper Scripts**
   - `run_agg_topk_debug.sh` - Easy test runner with multiple modes

3. **Documentation**
   - `AGGREGATION_TOPK_DEBUG_README.md` - Main overview
   - `LOGGING_GUIDE.md` - Complete logging reference ⭐
   - `AGGREGATION_TOPK_DEBUG_GUIDE.md` - Detailed debugging guide
   - `CUSTOM_INSTRUMENTATION_EXAMPLE.md` - How to add more logging
   - `QUICK_REFERENCE.md` - Cheat sheet
   - `SUMMARY.md` - This file

## Key Features

### 1. Comprehensive Logging

Every major operation is logged with:
- **Consistent prefixes** for easy filtering
- **Memory tracking** before and after operations
- **Batch flow tracking** between operators
- **EmitTo call tracking** to understand when and why groups are emitted
- **Performance metrics** (row counts, replacements, memory deltas)

### 2. Multiple Logging Levels

```bash
./run_agg_topk_debug.sh info     # Clean, high-level view
./run_agg_topk_debug.sh debug    # Detailed operations
./run_agg_topk_debug.sh trace    # Everything (very verbose)
./run_agg_topk_debug.sh focused  # Just AGG + TopK
./run_agg_topk_debug.sh memory   # Memory operations
```

### 3. Easy Filtering

```bash
# See only aggregation
grep '\[AGG-'

# See only TopK
grep '\[TOPK-'

# See batch flow
grep -E '(Returning batch|Receiving batch)'

# See EmitTo calls
grep 'emit_to='
```

## Understanding the Flow

### Phase 1: Aggregation Builds Groups
```
[AGG-FINAL] Received input batch: rows=8192
[AGG-BATCH] Created 1523 new groups
[AGG-FINAL] After aggregation: total_groups=1523, memory=245KB
```

### Phase 2: Emission Triggered
```
[AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10)
[MEMORY] BEFORE emit(All): total=289KB, num_groups=10
[MEMORY] AFTER emit(All): emitted_rows=10, memory_freed=285KB
```

### Phase 3: Output Production
```
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=10
```

### Phase 4: TopK Processing
```
[TOPK-INSERT] ▼ Receiving batch from aggregation: rows=10
[TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB
```

### Phase 5: Final Results
```
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

## What You Can Learn

### 1. Group Storage
- How groups are created and stored in `group_values`
- When new groups are discovered vs existing groups updated
- Memory usage of group storage

### 2. Emission Triggers
- **Soft limit**: When LIMIT clause triggers early emission
- **Group ordering**: When ordered input allows incremental emission
- **Memory pressure**: When memory limit forces early emission
- **Input done**: When all input is consumed

### 3. EmitTo Variants
- `EmitTo::All` - Emit all groups
- `EmitTo::First(n)` - Emit first n groups (ordered input)
- `EmitTo::NextBlock` - Emit next block (blocked optimization)

### 4. TopK Behavior
- How the heap maintains top K rows
- When rows are added vs rejected
- When compaction happens to save memory
- When early termination is possible

### 5. Memory Flow
- Memory growth as groups accumulate
- Memory freed when groups are emitted
- Memory usage in TopK heap and store
- Memory saved by compaction

## Debugging Scenarios

### High Memory Usage
**Symptoms**: Large memory values in logs
**Look for**: 
- `[MEMORY]` logs showing high totals
- `[AGG-EARLY-EMIT]` indicating memory pressure
- Large `total_groups` values

### Slow Performance
**Symptoms**: Long execution time
**Look for**:
- High `replacements_total` in TopK
- Frequent `[TOPK-COMPACT]` operations
- Many small batches being emitted

### Incorrect Results
**Symptoms**: Wrong output
**Look for**:
- Group count mismatches
- Unexpected `emit_to` values
- Early termination when it shouldn't happen

## Next Steps

1. **Run the test** to see the basic flow
   ```bash
   ./run_agg_topk_debug.sh
   ```

2. **Read LOGGING_GUIDE.md** to understand all the log messages

3. **Experiment** with different queries:
   - Change the LIMIT value
   - Add/remove ORDER BY
   - Use different aggregate functions
   - Try different data sizes

4. **Add custom logging** if you need more detail (see CUSTOM_INSTRUMENTATION_EXAMPLE.md)

5. **Use the debugger** for deep inspection:
   ```bash
   ./run_agg_topk_debug.sh lldb
   ```

## Important Notes

### Data Path
The test expects data at:
```
/Users/abandeji/Public/work-dump/clickbench_data/partitioned
```

If your data is elsewhere, edit `datafusion/core/tests/aggregation_topk_debug.rs` and update the path.

### Reverting Changes
To remove all the logging from source files:
```bash
git checkout datafusion/physical-plan/src/aggregates/row_hash.rs
git checkout datafusion/physical-plan/src/topk/mod.rs
```

The test files and documentation can be kept or removed as needed.

## Questions?

- **What do the log prefixes mean?** → See LOGGING_GUIDE.md
- **How do I run the test?** → See AGGREGATION_TOPK_DEBUG_GUIDE.md
- **How do I add more logging?** → See CUSTOM_INSTRUMENTATION_EXAMPLE.md
- **Quick reference?** → See QUICK_REFERENCE.md

## Files Overview

```
.
├── datafusion/
│   ├── core/tests/
│   │   └── aggregation_topk_debug.rs          # Test file
│   └── physical-plan/src/
│       ├── aggregates/row_hash.rs             # Modified with logging
│       └── topk/mod.rs                        # Modified with logging
│
├── run_agg_topk_debug.sh                      # Test runner script
│
├── SUMMARY.md                                 # This file
├── QUICK_REFERENCE.md                         # Cheat sheet
├── FLOW_DIAGRAM.md                            # Visual flow diagrams
├── LOGGING_GUIDE.md                           # Complete log reference ⭐
├── AGGREGATION_TOPK_DEBUG_README.md           # Main overview
├── AGGREGATION_TOPK_DEBUG_GUIDE.md            # Detailed guide
└── CUSTOM_INSTRUMENTATION_EXAMPLE.md          # Add your own logs
```

## Success Criteria

You'll know the setup is working when you:

1. ✅ Can run the test successfully
2. ✅ See log messages with `[AGG-*]` and `[TOPK-*]` prefixes
3. ✅ Can track a batch from input → aggregation → TopK → output
4. ✅ Understand when and why EmitTo is called
5. ✅ Can see memory usage and how it changes

Happy debugging! 🐛🔍
