# Documentation Index

Complete index of all debugging documentation and resources.

## 🎯 Start Here

**New to this setup?** → [README_DEBUG_SETUP.md](README_DEBUG_SETUP.md)

**Want to see what was created?** → [COMPLETE_SETUP_SUMMARY.md](COMPLETE_SETUP_SUMMARY.md)

**Want a quick overview?** → [SUMMARY.md](SUMMARY.md)

**Need a cheat sheet?** → [QUICK_REFERENCE.md](QUICK_REFERENCE.md)

## 📖 Core Documentation

### Getting Started

| File | Description | Read Time |
|------|-------------|-----------|
| [README_DEBUG_SETUP.md](README_DEBUG_SETUP.md) | Main entry point | 2 min |
| [SUMMARY.md](SUMMARY.md) | Complete overview | 10 min |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Cheat sheet | 2 min |

### Understanding the System

| File | Description | Read Time |
|------|-------------|-----------|
| [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) | Visual architecture diagrams | 15 min |
| [LOGGING_GUIDE.md](LOGGING_GUIDE.md) | Complete log reference | 20 min |
| [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md) | Detailed debugging guide | 30 min |

### Advanced Topics

| File | Description | Read Time |
|------|-------------|-----------|
| [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md) | Add your own logging | 15 min |
| [AGGREGATION_TOPK_DEBUG_README.md](AGGREGATION_TOPK_DEBUG_README.md) | Original comprehensive guide | 25 min |

## 🔧 Implementation Files

### Test and Scripts

| File | Purpose |
|------|---------|
| `datafusion/core/tests/aggregation_topk_debug.rs` | Test implementation |
| `run_agg_topk_debug.sh` | Test runner script |

### Modified Source Files

| File | Changes |
|------|---------|
| `datafusion/physical-plan/src/aggregates/row_hash.rs` | Added aggregation logging |
| `datafusion/physical-plan/src/topk/mod.rs` | Added TopK logging |

## 📚 Documentation by Purpose

### I want to...

#### Run the test
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - See "Run the Test" section

#### Understand log messages
→ [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - Complete log reference

#### See the architecture
→ [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) - Visual diagrams

#### Debug a specific issue
→ [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md) - See "Troubleshooting" section

#### Add more logging
→ [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md) - Examples and patterns

#### Understand EmitTo calls
→ [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - See "EmitTo Calls" section
→ [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) - See "EmitTo Decision Flow"

#### Track memory usage
→ [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - See "Memory Tracking" section
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - See "Quick Filters"

#### Understand batch flow
→ [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) - See "Detailed Data Flow"
→ [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - See "Understanding the Complete Flow"

## 🎓 Learning Paths

### Path 1: Quick Start (15 minutes)
1. [README_DEBUG_SETUP.md](README_DEBUG_SETUP.md) - Overview
2. Run `./run_agg_topk_debug.sh`
3. [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Understand output

### Path 2: Deep Understanding (1 hour)
1. [SUMMARY.md](SUMMARY.md) - Complete overview
2. [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md) - Visual understanding
3. [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - Log interpretation
4. Run test with different modes
5. [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md) - Advanced topics

### Path 3: Custom Development (2 hours)
1. Complete Path 2
2. [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md) - Learn patterns
3. Add your own logging
4. Experiment with modifications

## 🔍 Quick Lookups

### Log Prefixes
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - "Log Prefixes Cheat Sheet"

### Filtering Commands
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - "Quick Filters"
→ [LOGGING_GUIDE.md](LOGGING_GUIDE.md) - "Filtering Logs"

### Common Issues
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - "Common Issues"
→ [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md) - "Troubleshooting"

### Running Options
→ [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - "Run the Test"
→ [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md) - "Running the Test"

## 📊 Documentation Statistics

| Category | Files | Total Pages (est.) |
|----------|-------|-------------------|
| Getting Started | 3 | 15 |
| Core Guides | 3 | 65 |
| Advanced | 2 | 40 |
| **Total** | **8** | **120** |

## 🗺️ Documentation Map

```
README_DEBUG_SETUP.md (Entry Point)
    │
    ├─→ SUMMARY.md (Overview)
    │   ├─→ QUICK_REFERENCE.md (Cheat Sheet)
    │   ├─→ FLOW_DIAGRAM.md (Visuals)
    │   └─→ LOGGING_GUIDE.md (Log Reference)
    │
    ├─→ AGGREGATION_TOPK_DEBUG_GUIDE.md (Detailed Guide)
    │   └─→ CUSTOM_INSTRUMENTATION_EXAMPLE.md (Advanced)
    │
    └─→ AGGREGATION_TOPK_DEBUG_README.md (Original Guide)
```

## 🎯 By Experience Level

### Beginner
1. [README_DEBUG_SETUP.md](README_DEBUG_SETUP.md)
2. [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
3. [FLOW_DIAGRAM.md](FLOW_DIAGRAM.md)

### Intermediate
1. [SUMMARY.md](SUMMARY.md)
2. [LOGGING_GUIDE.md](LOGGING_GUIDE.md)
3. [AGGREGATION_TOPK_DEBUG_GUIDE.md](AGGREGATION_TOPK_DEBUG_GUIDE.md)

### Advanced
1. [CUSTOM_INSTRUMENTATION_EXAMPLE.md](CUSTOM_INSTRUMENTATION_EXAMPLE.md)
2. Source code modifications
3. Custom debugging strategies

## 🔗 External Resources

- **DataFusion Docs**: https://datafusion.apache.org/
- **Source Code**:
  - `datafusion/physical-plan/src/aggregates/row_hash.rs`
  - `datafusion/physical-plan/src/topk/mod.rs`

## 📝 Notes

- All documentation assumes you're in the DataFusion root directory
- Log examples use the ClickBench dataset
- Timing estimates are approximate
- Some files reference each other - follow the links!

## 🆘 Still Lost?

1. Start with [README_DEBUG_SETUP.md](README_DEBUG_SETUP.md)
2. Run the test: `./run_agg_topk_debug.sh`
3. Read [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
4. If still confused, read [SUMMARY.md](SUMMARY.md)

---

**Last Updated**: December 2024
**Purpose**: Debug DataFusion aggregation and TopK interaction
