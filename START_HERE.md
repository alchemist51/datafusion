# 🎯 START HERE

## What Is This?

A complete debugging setup to understand how DataFusion's aggregation and TopK operators work together.

## 🚀 Quick Start (Choose Your Path)

### Path 1: Just Run It (5 minutes)
```bash
./run_agg_topk_debug.sh
```
Then read [GET_STARTED.md](GET_STARTED.md)

### Path 2: Understand First (15 minutes)
1. Read [COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)
2. Run `./run_agg_topk_debug.sh`
3. Read [LOGGING_GUIDE.md](LOGGING_GUIDE.md)

### Path 3: Deep Dive (1 hour)
1. Read [SUMMARY.md](SUMMARY.md)
2. Read [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)
3. Run test with different modes
4. Read [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)

## 📚 All Documentation

| File | Purpose | Time |
|------|---------|------|
| **[GET_STARTED.md](GET_STARTED.md)** | 5-minute quick start | 5 min |
| **[COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)** | What was created | 10 min |
| **[SUMMARY.md](SUMMARY.md)** | Complete overview | 10 min |
| **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** | Cheat sheet | 2 min |
| **[FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)** | Visual diagrams | 15 min |
| **[LOGGING_GUIDE.md](LOGGING_GUIDE.md)** | Log reference ⭐ | 20 min |
| **[AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)** | Detailed guide | 30 min |
| **[CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)** | Add logging | 15 min |
| **[INDEX.md](INDEX.md)** | Full index | 2 min |

## 🎓 Recommended Reading Order

### For Beginners
1. [GET_STARTED.md](GET_STARTED.md) ← Start here!
2. [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
3. [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)

### For Intermediate Users
1. [COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)
2. [LOGGING_GUIDE.md](LOGGING_GUIDE.md)
3. [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)

### For Advanced Users
1. [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)
2. Modify source files
3. Create custom debugging strategies

## 🔍 What You'll Learn

- ✅ How groups are stored in aggregation
- ✅ When and why EmitTo is called
- ✅ How batches flow between operators
- ✅ Memory usage at each stage
- ✅ TopK heap operations
- ✅ Performance bottlenecks

## 💡 Quick Tips

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

## 📊 What Was Created

- ✅ 11 documentation files (~150 pages)
- ✅ 1 test file with your query
- ✅ 1 test runner script
- ✅ Comprehensive logging in 2 source files

## 🎯 Your Goal

Understand this flow:
```
Input Data
    ↓
Aggregation (groups by WatchID)
    ↓
EmitTo (triggered by LIMIT)
    ↓
TopK (maintains top 10)
    ↓
Final Results
```

## 🆘 Need Help?

1. **Quick question?** → [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
2. **Log message unclear?** → [LOGGING_GUIDE.md](LOGGING_GUIDE.md)
3. **Architecture question?** → [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)
4. **Debugging issue?** → [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)
5. **Want more logging?** → [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)

## 🗺️ Navigation

```
START_HERE.md (You are here!)
    │
    ├─→ GET_STARTED.md (5-minute start)
    │
    ├─→ COMPLETE_SETUP_SUMMARY.md (What was created)
    │
    ├─→ SUMMARY.md (Complete overview)
    │   ├─→ QUICK_REFERENCE.md
    │   ├─→ FLOW_DIAGRAM.md
    │   └─→ LOGGING_GUIDE.md ⭐
    │
    ├─→ AGGREGATION_TOPK_DEBUG_GUIDE.md
    │   └─→ CUSTOM_INSTRUMENTATION_EXAMPLE.md
    │
    └─→ INDEX.md (Full documentation index)
```

## ⚡ TL;DR

```bash
# 1. Run this
./run_agg_topk_debug.sh

# 2. Read this
cat GET_STARTED.md

# 3. Understand this
cat LOGGING_GUIDE.md

# Done! 🎉
```

---

**Ready?** → [GET_STARTED.md](GET_STARTED.md)

**Want overview first?** → [COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)

**Need full index?** → [INDEX.md](INDEX.md)
