# Aggregation + TopK Debugging Setup

Complete debugging setup for understanding DataFusion's GroupedHashAggregateStream and TopK interaction.

## 🚀 Quick Start

```bash
# Run the test
./run_agg_topk_debug.sh

# See the logs with prefixes like [AGG-*] and [TOPK-*]
```

## 📚 Documentation

Start here based on what you need:

| Document | Purpose | When to Read |
|----------|---------|--------------|
| **[SUMMARY.md](SUMMARY.md)** | Overview of everything | Start here first |
| **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** | Cheat sheet | Quick lookup |
| **[FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)** | Visual diagrams | Understand architecture |
| **[LOGGING_GUIDE.md](LOGGING_GUIDE.md)** | Complete log reference | Interpret log messages |
| **[AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)** | Detailed debugging guide | Deep dive |
| **[CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)** | Add your own logs | Need more detail |

## 🎯 What This Does

Helps you understand:

1. **How groups are stored** in the hash aggregation
2. **When groups are emitted** (EmitTo calls)
3. **How batches flow** from aggregation to TopK
4. **Memory usage** at each stage
5. **TopK heap operations** (add/reject/compact)

## 📝 Example Query

```sql
SELECT "WatchID", 
       MIN("ResolutionWidth"), 
       MAX("ResolutionWidth"), 
       SUM("IsRefresh")
FROM hits 
GROUP BY "WatchID" 
ORDER BY "WatchID" DESC 
LIMIT 10
```

## 🔍 What You'll See

```
[AGG-FINAL] Received input batch: rows=8192, current_groups=0
[AGG-BATCH] Created 1523 new groups (total now: 1523)
[AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10)
[MEMORY] BEFORE emit(All): total=289KB, num_groups=10
[MEMORY] AFTER emit(All): emitted_rows=10, memory_freed=285KB
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=10
[TOPK-INSERT] ▼ Receiving batch from aggregation: rows=10
[TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

## 🛠️ Modified Files

- `datafusion/physical-plan/src/aggregates/row_hash.rs` - Aggregation logging
- `datafusion/physical-plan/src/topk/mod.rs` - TopK logging

## 🔄 Reverting Changes

```bash
git checkout datafusion/physical-plan/src/aggregates/row_hash.rs
git checkout datafusion/physical-plan/src/topk/mod.rs
```

## 💡 Tips

1. Start with **SUMMARY.md** for the big picture
2. Use **QUICK_REFERENCE.md** for quick lookups
3. Read **LOGGING_GUIDE.md** to understand all log messages
4. Check **FLOW_DIAGRAM.md** for visual understanding
5. Run with `./run_agg_topk_debug.sh debug` for more detail

## 🎓 Learning Path

```
1. Read SUMMARY.md (5 min)
   ↓
2. Run ./run_agg_topk_debug.sh (1 min)
   ↓
3. Review LOGGING_GUIDE.md (10 min)
   ↓
4. Study FLOW_DIAGRAM.md (10 min)
   ↓
5. Experiment with different queries
   ↓
6. Add custom logging if needed
```

## 📊 Log Prefixes

- `[AGG-PARTIAL]` - Partial aggregation
- `[AGG-FINAL]` - Final aggregation
- `[AGG-BATCH]` - Group creation
- `[AGG-OUTPUT]` - Output production
- `[MEMORY]` - Memory tracking
- `[TOPK-INSERT]` - TopK batch insertion
- `[TOPK-EMIT]` - TopK final emission

## 🐛 Common Issues

### Data path not found
Edit `datafusion/core/tests/aggregation_topk_debug.rs` and update the path.

### Too much output
Use `./run_agg_topk_debug.sh info` for cleaner output.

### Need more detail
Use `./run_agg_topk_debug.sh debug` or add custom logging.

## 📞 Need Help?

1. Check **LOGGING_GUIDE.md** for log message meanings
2. See **AGGREGATION_TOPK_DEBUG_GUIDE.md** for debugging tips
3. Review **FLOW_DIAGRAM.md** for architecture understanding
4. Add custom logging using **CUSTOM_INSTRUMENTATION_EXAMPLE.md**

---

**Created for debugging DataFusion aggregation and TopK interaction**
