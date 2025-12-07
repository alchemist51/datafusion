# Aggregation + TopK Debugging Setup

Complete debugging environment for understanding DataFusion's aggregation and TopK interaction.

## 🚀 Quick Start

```bash
./run_agg_topk_debug.sh
```

## 📖 Documentation

**New here?** → [START_HERE.md](START_HERE.md)

**5-minute start?** → [GET_STARTED.md](GET_STARTED.md)

**Full index?** → [INDEX.md](INDEX.md)

## 📚 Key Documents

| Document | Purpose |
|----------|---------|
| [START_HERE.md](START_HERE.md) | Main entry point |
| [GET_STARTED.md](GET_STARTED.md) | 5-minute quick start |
| [LOGGING_GUIDE.md](LOGGING_GUIDE.md) | Complete log reference ⭐ |
| [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) | Visual diagrams |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Cheat sheet |

## 🎯 What This Does

Helps you understand:
- How groups are stored in aggregation
- When and why EmitTo is called
- How batches flow between operators
- Memory usage at each stage
- TopK heap operations

## 📊 Example Output

```
[AGG-FINAL] Received input batch: rows=8192
[AGG-BATCH] Created 1523 new groups
[AGG-FINAL] Hit soft group limit: groups=10
[MEMORY] BEFORE emit(All): total=289KB
[MEMORY] AFTER emit: emitted_rows=10, memory_freed=285KB
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=10
[TOPK-INSERT] ▼ Receiving batch: rows=10, heap=0/10
[TOPK-INSERT] ✓ Batch processed: heap=10/10
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

## 🔧 What Was Created

- ✅ 14 documentation files (~150 pages)
- ✅ 1 test file with your query
- ✅ 1 test runner script
- ✅ Comprehensive logging in source files

## 💡 Quick Commands

```bash
# Run with clean output
./run_agg_topk_debug.sh

# Run with detailed output
./run_agg_topk_debug.sh debug

# See only aggregation
./run_agg_topk_debug.sh | grep '\[AGG-'

# See only TopK
./run_agg_topk_debug.sh | grep '\[TOPK-'
```

## 🗺️ Navigation

```
README.md (You are here!)
    ↓
START_HERE.md (Choose your path)
    ↓
GET_STARTED.md (5-minute start)
    ↓
LOGGING_GUIDE.md (Understand logs)
    ↓
FLOW_DIAGRAM.md (See architecture)
```

## 🆘 Need Help?

- **Quick question?** → [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
- **Log unclear?** → [LOGGING_GUIDE.md](LOGGING_GUIDE.md)
- **Architecture?** → [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)
- **Full index?** → [INDEX.md](INDEX.md)

## 🎓 Learning Paths

### Beginner (15 min)
1. [START_HERE.md](START_HERE.md)
2. Run `./run_agg_topk_debug.sh`
3. [GET_STARTED.md](GET_STARTED.md)

### Intermediate (1 hour)
1. [COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)
2. [LOGGING_GUIDE.md](LOGGING_GUIDE.md)
3. [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)

### Advanced (2+ hours)
1. [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)
2. [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)
3. Add your own logging

## ✨ Features

- Comprehensive logging with consistent prefixes
- Multiple logging levels (info, debug, trace)
- Easy filtering by component
- Visual architecture diagrams
- Complete documentation
- Debugging scenarios
- Custom instrumentation examples

## 🎯 Success Criteria

You'll know it's working when you can:
- ✅ Run the test successfully
- ✅ See and understand log messages
- ✅ Track batches through the system
- ✅ Explain EmitTo triggers
- ✅ Monitor memory usage
- ✅ Debug issues independently

---

**Ready to start?** → [START_HERE.md](START_HERE.md)

**Just want to run it?** → `./run_agg_topk_debug.sh`

**Need the full picture?** → [FINAL_SUMMARY.md](FINAL_SUMMARY.md)
