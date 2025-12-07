# Get Started in 5 Minutes

Quick guide to start debugging aggregation and TopK interaction.

## Step 1: Run the Test (1 minute)

```bash
./run_agg_topk_debug.sh
```

You'll see output like:
```
[AGG-FINAL] Received input batch: rows=8192
[AGG-BATCH] Created 1523 new groups
[TOPK-INSERT] ▼ Receiving batch: rows=10
[TOPK-EMIT] ✓ Emission complete
```

## Step 2: Understand the Prefixes (2 minutes)

| Prefix | What It Means |
|--------|---------------|
| `[AGG-*]` | Aggregation operations |
| `[TOPK-*]` | TopK operations |
| `[MEMORY]` | Memory tracking |
| `▼` | Receiving data |
| `✓` | Operation complete |

## Step 3: Follow the Flow (2 minutes)

Watch for this sequence:

```
1. [AGG-*] Received input batch
   ↓
2. [AGG-BATCH] Created N new groups
   ↓
3. [MEMORY] BEFORE emit
   ↓
4. [MEMORY] AFTER emit (memory freed)
   ↓
5. [AGG-OUTPUT] ✓ Returning batch to upstream
   ↓
6. [TOPK-INSERT] ▼ Receiving batch
   ↓
7. [TOPK-INSERT] ✓ Batch processed
   ↓
8. [TOPK-EMIT] ✓ Emission complete
```

## What's Happening?

1. **Aggregation** groups rows by WatchID
2. **Soft limit** (LIMIT 10) triggers early emission
3. **Memory freed** when groups emitted
4. **TopK** receives the batch
5. **Heap** maintains top 10 rows
6. **Final results** emitted

## Next Steps

### Want more detail?
```bash
./run_agg_topk_debug.sh debug
```

### Want to understand the logs?
Read [LOGGING_GUIDE.md](LOGGING_GUIDE.md)

### Want to see the architecture?
Read [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)

### Want the complete picture?
Read [SUMMARY.md](SUMMARY.md)

## Quick Filters

```bash
# See only aggregation
./run_agg_topk_debug.sh | grep '\[AGG-'

# See only TopK
./run_agg_topk_debug.sh | grep '\[TOPK-'

# See only memory
./run_agg_topk_debug.sh | grep '\[MEMORY\]'
```

## Common Questions

**Q: Where's my data?**
A: Update the path in `datafusion/core/tests/aggregation_topk_debug.rs`

**Q: Too much output?**
A: Use `./run_agg_topk_debug.sh info` or filter with grep

**Q: What does EmitTo mean?**
A: It's when aggregation sends groups to the next operator. See [LOGGING_GUIDE.md](LOGGING_GUIDE.md)

**Q: Why is memory freed?**
A: After emitting groups, aggregation clears its internal state

**Q: What's the heap?**
A: TopK uses a heap to efficiently maintain the top K rows

## That's It!

You now understand the basics. For more detail, check out:

- [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - Complete log reference
- [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) - Visual diagrams
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Cheat sheet
- [INDEX.md](INDEX.md) - Full documentation index

Happy debugging! 🐛🔍
