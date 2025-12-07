# Aggregation + TopK Interaction Debug Test

This test helps you understand how DataFusion's `GroupedHashAggregateStream` and `TopK` operators interact when executing queries with `GROUP BY`, `ORDER BY`, and `LIMIT`.

## Quick Start

```bash
# Run the test with debug output
./run_agg_topk_debug.sh

# Or manually
RUST_LOG=debug cargo test --test aggregation_topk_debug -- --nocapture
```

## What This Tests

The test executes this query against your ClickBench data:

```sql
SELECT "WatchID", 
       MIN("ResolutionWidth") as min_width, 
       MAX("ResolutionWidth") as max_width, 
       SUM("IsRefresh") as sum_refresh
FROM hits 
GROUP BY "WatchID" 
ORDER BY "WatchID" DESC 
LIMIT 10
```

## Files Created

1. **`datafusion/core/tests/aggregation_topk_debug.rs`**
   - The actual test file
   - Executes the query and logs the execution flow

2. **`run_agg_topk_debug.sh`**
   - Convenient script to run the test with different logging levels
   - Supports: debug, trace, info, focused, memory, lldb modes

3. **`AGGREGATION_TOPK_DEBUG_GUIDE.md`**
   - Comprehensive guide on running and interpreting the test
   - Explains what to look for in the output
   - Debugging tips and techniques

4. **`LOGGING_GUIDE.md`** ⭐ **START HERE**
   - Complete reference for all logging added to the codebase
   - Explains every log prefix and message
   - Shows how to filter and interpret logs
   - Includes debugging scenarios and examples

5. **`CUSTOM_INSTRUMENTATION_EXAMPLE.md`**
   - Shows how to add your own logging to the source code
   - Examples of tracking memory, groups, and batch flow
   - Performance profiling integration

## Understanding the Execution Flow

### 1. Aggregation Phase (GroupedHashAggregateStream)

```
Input Batches → Group Assignment → Accumulation → Emission
                     ↓                   ↓            ↓
                group_values      accumulators   RecordBatch
```

**Key Points:**
- Groups are stored in `group_values` (hash table)
- Each group has associated accumulator state (MIN, MAX, SUM)
- Groups are emitted as `RecordBatch`es when:
  - Memory pressure requires it
  - All input has been consumed
  - Soft limit on groups is reached

### 2. TopK Phase

```
Aggregated Batches → Sort Key Extraction → Heap Management → Final Emission
                           ↓                      ↓               ↓
                      Row Converter          BinaryHeap      Top K Results
```

**Key Points:**
- Maintains a heap of the top K rows (K=10 in this query)
- Uses row format for efficient comparison
- Can terminate early if input is partially sorted
- Compacts storage to reduce memory usage

### 3. Memory Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Memory Lifecycle                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Aggregation:                                                │
│    ↑ Grows as groups accumulate                             │
│    ↓ Drops when groups are emitted                          │
│    ⚠ May spill to disk if memory limit exceeded             │
│                                                               │
│  TopK:                                                        │
│    → Bounded by K × row_size                                │
│    → Stores original RecordBatches                          │
│    → Compacts when too many unused rows                     │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Running Options

### Basic Debug
```bash
./run_agg_topk_debug.sh debug
```
Shows major operations and state changes.

### Trace Level (Very Verbose)
```bash
./run_agg_topk_debug.sh trace
```
Shows every detail including per-row operations.

### Focused Logging
```bash
./run_agg_topk_debug.sh focused
```
Only shows aggregation and TopK components.

### Memory Tracking
```bash
./run_agg_topk_debug.sh memory
```
Focuses on memory pool operations and reservations.

### Interactive Debugging
```bash
./run_agg_topk_debug.sh lldb
```
Launches lldb debugger with the test binary.

## What to Look For

### In the Logs

1. **Group Creation**
   ```
   [AGG-GROUPS] Batch processed: input_rows=8192, unique_groups=1523, memory=245KB
   ```

2. **Memory Changes**
   ```
   [AGG-MEMORY] Reservation updated: new_size=512KB, change=+267KB
   ```

3. **Group Emission**
   ```
   [AGG-EMIT] Emitting batch: rows=1523, groups_remaining=0, memory_before=512KB
   ```

4. **TopK Processing**
   ```
   [TOPK-INSERT] Receiving batch: rows=1523, current_heap_size=0/10, memory=2KB
   [TOPK-INSERT] Batch processed: heap_size=10/10, replacements=1513
   ```

5. **TopK Compaction**
   ```
   [TOPK-COMPACT] Compacting: old_batches=3, new_batches=1, memory_saved=128KB
   ```

### Performance Metrics

At the end of execution, you'll see:
- Total execution time
- Number of batches processed
- Memory usage patterns
- Row replacement counts (TopK)

## Debugging Specific Issues

### Issue: High Memory Usage

**Look for:**
- Number of unique groups (high cardinality?)
- Frequency of emissions (too infrequent?)
- Spilling behavior (is it spilling to disk?)

**Solutions:**
- Reduce batch size
- Enable/tune spilling
- Increase memory limit

### Issue: Slow Performance

**Look for:**
- Large number of row replacements in TopK
- Frequent compaction in TopK
- Many small batches being emitted

**Solutions:**
- Check if early termination is working
- Tune batch sizes
- Consider different aggregation strategy

### Issue: Incorrect Results

**Look for:**
- Group count mismatches
- Accumulator state issues
- TopK heap ordering problems

**Solutions:**
- Add detailed logging (see CUSTOM_INSTRUMENTATION_EXAMPLE.md)
- Use debugger to inspect state
- Verify input data

## Next Steps

1. **Run the basic test** to see the overall flow
2. **Review the logs** to understand the execution
3. **Add custom instrumentation** if you need more detail
4. **Use the debugger** to step through specific operations
5. **Experiment** with different queries and data

## Modifying the Test

To test different scenarios, edit `datafusion/core/tests/aggregation_topk_debug.rs`:

```rust
// Change the query
let sql = r#"
    SELECT "WatchID", COUNT(*) as cnt
    FROM hits 
    GROUP BY "WatchID" 
    ORDER BY cnt DESC 
    LIMIT 100
"#;

// Change the data path
let data_path = "/your/path/to/data";

// Change session config
let config = SessionConfig::new()
    .with_batch_size(4096)  // Smaller batches
    .with_target_partitions(4); // More parallelism
```

## Troubleshooting

### Can't find data file
Update the path in the test:
```rust
let data_path = "/Users/abandeji/Public/work-dump/clickbench_data/partitioned";
```

### Too much output
Use focused logging:
```bash
./run_agg_topk_debug.sh focused
```

### Test hangs
Check if the data path is correct and accessible.

### Compilation errors
Make sure you're in the DataFusion root directory:
```bash
cd /path/to/datafusion
cargo test --test aggregation_topk_debug
```

## Additional Resources

- **DataFusion Docs**: https://datafusion.apache.org/
- **Source Code**:
  - `datafusion/physical-plan/src/aggregates/row_hash.rs`
  - `datafusion/physical-plan/src/topk/mod.rs`
- **Related Tests**:
  - `datafusion/core/tests/sql/aggregates.rs`
  - `datafusion/core/tests/sql/limit.rs`

## Questions?

If you have questions or find issues:
1. Check the detailed guides (AGGREGATION_TOPK_DEBUG_GUIDE.md)
2. Review the instrumentation examples (CUSTOM_INSTRUMENTATION_EXAMPLE.md)
3. Add more logging to understand the specific behavior
4. Use the debugger to inspect state at runtime

Happy debugging! 🐛🔍
