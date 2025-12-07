# Flow Diagram: Aggregation + TopK Interaction

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Query Execution                              │
│                                                                       │
│  SELECT "WatchID", MIN(...), MAX(...), SUM(...)                     │
│  FROM hits                                                            │
│  GROUP BY "WatchID"                                                   │
│  ORDER BY "WatchID" DESC                                             │
│  LIMIT 10                                                             │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Physical Plan                                     │
│                                                                       │
│  GlobalLimitExec: skip=0, fetch=10                                   │
│    └─ TopKExec: k=10, order_by=[WatchID DESC]                       │
│         └─ AggregateExec: mode=Final, group_by=[WatchID]            │
│              └─ AggregateExec: mode=Partial, group_by=[WatchID]     │
│                   └─ ParquetExec                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Detailed Data Flow

```
┌──────────────────┐
│  Parquet Files   │
│  (Input Data)    │
└────────┬─────────┘
         │ RecordBatch (8192 rows)
         │
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Partial Aggregation (GroupedHashAggregateStream)       │
│                                                                       │
│  [AGG-PARTIAL] Received input batch: rows=8192                      │
│                                                                       │
│  ┌──────────────┐         ┌─────────────────┐                      │
│  │ group_values │◄────────┤  Accumulators   │                      │
│  │  (HashMap)   │         │  MIN, MAX, SUM  │                      │
│  │              │         │                  │                      │
│  │  WatchID →   │         │  [state arrays] │                      │
│  │  group_idx   │         │                  │                      │
│  └──────────────┘         └─────────────────┘                      │
│                                                                       │
│  [AGG-BATCH] Created 1523 new groups (total now: 1523)              │
│  [AGG-PARTIAL] After aggregation: total_groups=1523, memory=245KB   │
│                                                                       │
│  ... (more batches processed) ...                                    │
│                                                                       │
│  [MEMORY] BEFORE emit(All): total=289KB, num_groups=1523            │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  emit(EmitTo::All)                                           │   │
│  │  - Extract all group keys from group_values                  │   │
│  │  - Get partial state from each accumulator                   │   │
│  │  - Combine into RecordBatch                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                       │
│  [MEMORY] AFTER emit: emitted_rows=1523, memory_freed=285KB         │
└───────────────────────────────────┬───────────────────────────────────┘
                                    │ RecordBatch (1523 rows)
                                    │ [WatchID, MIN_state, MAX_state, SUM_state]
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Final Aggregation (GroupedHashAggregateStream)          │
│                                                                       │
│  [AGG-FINAL] Received input batch: rows=1523, current_groups=0      │
│                                                                       │
│  ┌──────────────┐         ┌─────────────────┐                      │
│  │ group_values │◄────────┤  Accumulators   │                      │
│  │  (HashMap)   │         │  MIN, MAX, SUM  │                      │
│  │              │         │  (merge mode)   │                      │
│  │  WatchID →   │         │                  │                      │
│  │  group_idx   │         │  [final values] │                      │
│  └──────────────┘         └─────────────────┘                      │
│                                                                       │
│  [AGG-BATCH] Created 1523 new groups (total now: 1523)              │
│                                                                       │
│  ... (more partial batches merged) ...                               │
│                                                                       │
│  [AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10)        │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  emit(EmitTo::All) - triggered by soft limit                 │   │
│  │  - Extract all group keys                                     │   │
│  │  - Evaluate final aggregate values                            │   │
│  │  - Combine into RecordBatch                                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                       │
│  [AGG-OUTPUT] ✓ Returning batch to upstream: rows=10                │
└───────────────────────────────────┬───────────────────────────────────┘
                                    │ RecordBatch (10 rows)
                                    │ [WatchID, MIN_val, MAX_val, SUM_val]
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        TopK Operator                                 │
│                                                                       │
│  [TOPK-INSERT] ▼ Receiving batch: rows=10, current_heap=0/10        │
│                                                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  For each row:                                                │  │
│  │    1. Convert to sort key (WatchID DESC)                      │  │
│  │    2. Compare with heap.max()                                 │  │
│  │    3. If better, add to heap (evict worst if full)            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                       │
│  ┌─────────────────┐         ┌──────────────────────┐              │
│  │  BinaryHeap     │         │  RecordBatchStore    │              │
│  │  (TopKRow)      │         │                       │              │
│  │                 │         │  Stores original      │              │
│  │  [sort keys]    │────────▶│  RecordBatches        │              │
│  │  max on top     │         │  referenced by heap   │              │
│  └─────────────────┘         └──────────────────────┘              │
│                                                                       │
│  [TOPK-INSERT] Processed rows: added=10, rejected=0                 │
│  [TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB            │
│                                                                       │
│  ... (more batches could arrive, but soft limit = 10) ...           │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  emit()                                                       │   │
│  │  - Sort heap contents (smallest to largest)                   │   │
│  │  - Interleave rows from stored batches                        │   │
│  │  - Return as RecordBatch stream                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                       │
│  [TOPK-EMIT] ✓ Emission complete: 1 output batches                  │
└───────────────────────────────────┬───────────────────────────────────┘
                                    │ RecordBatch (10 rows, sorted)
                                    │ [WatchID, MIN_val, MAX_val, SUM_val]
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      GlobalLimitExec                                 │
│                      (pass through, already limited)                 │
└───────────────────────────────────┬───────────────────────────────────┘
                                    │
                                    ▼
                            ┌───────────────┐
                            │ Final Results │
                            │   (10 rows)   │
                            └───────────────┘
```

## Memory Flow Diagram

```
Memory Usage Over Time:

Partial Aggregation:
│
│  ▲ Memory
│  │
│  │     ┌─────────────────┐
│  │     │  Accumulating   │
│  │     │  groups         │
│  │    ╱                  │
│  │   ╱                   │
│  │  ╱                    │
│  │ ╱                     │
│  │╱                      └─────────────────┐
│  └────────────────────────────────────────┴──────▶ Time
│  Input  Input  Input  Input  emit()  Input  Input
│  batch  batch  batch  batch  ↓       batch  batch
│                               └─ Memory freed

Final Aggregation:
│
│  ▲ Memory
│  │
│  │     ┌──────┐
│  │     │ Soft │
│  │     │limit │
│  │    ╱│ hit  │
│  │   ╱ │      │
│  │  ╱  │      │
│  │ ╱   │      │
│  │╱    └──────┘
│  └────────────────────────────────────────────────▶ Time
│  Partial Partial Partial emit()
│  batch   batch   batch   ↓
│                           └─ All groups emitted

TopK:
│
│  ▲ Memory
│  │
│  │  ┌─────────────────────────────────────────────┐
│  │  │  Bounded by K (constant after heap fills)   │
│  │  │                                              │
│  │ ╱│                                              │
│  │╱ │                                              │
│  └──┴──────────────────────────────────────────────▶ Time
│     Batch Batch Batch ... Batch emit()
│     (heap fills quickly, then stable)
```

## EmitTo Decision Flow

```
┌─────────────────────────────────────────────────────────────┐
│  New batch arrives at GroupedHashAggregateStream            │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │ group_aggregate_batch│
              │ (process batch)      │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │ Check emission       │
              │ conditions           │
              └──────────┬───────────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
         ▼               ▼               ▼
┌────────────────┐ ┌──────────┐ ┌─────────────────┐
│ Soft limit hit?│ │ Ordering │ │ Memory pressure?│
│ (LIMIT clause) │ │ allows   │ │ (Partial only)  │
└────────┬───────┘ │ emit?    │ └────────┬────────┘
         │         └────┬─────┘          │
         │              │                │
         │ YES          │ YES            │ YES
         │              │                │
         ▼              ▼                ▼
    ┌────────┐    ┌─────────┐    ┌──────────────┐
    │emit(All)│    │emit(    │    │emit(First(n))│
    │        │    │First(n))│    │              │
    └────┬───┘    └────┬────┘    └──────┬───────┘
         │             │                 │
         └─────────────┼─────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │ Produce output │
              │ batch          │
              └────────┬───────┘
                       │
                       ▼
              ┌────────────────┐
              │ Return to      │
              │ upstream       │
              │ (TopK)         │
              └────────────────┘
```

## Log Message Timeline

```
Time  │ Component      │ Log Message
──────┼────────────────┼─────────────────────────────────────────────
0ms   │ AGG-PARTIAL    │ Received input batch: rows=8192
1ms   │ AGG-BATCH      │ Created 1523 new groups
2ms   │ AGG-PARTIAL    │ After aggregation: total_groups=1523
      │                │
10ms  │ AGG-PARTIAL    │ Received input batch: rows=8192
11ms  │ AGG-BATCH      │ Created 245 new groups (total: 1768)
      │                │
... (more batches) ...
      │                │
50ms  │ AGG-PARTIAL    │ Input done, emitting all groups
51ms  │ MEMORY         │ BEFORE emit(All): total=289KB, groups=1768
52ms  │ MEMORY         │ AFTER emit: emitted_rows=1768, freed=285KB
53ms  │ AGG-OUTPUT     │ ✓ Returning batch to upstream: rows=1768
      │                │
54ms  │ AGG-FINAL      │ Received input batch: rows=1768
55ms  │ AGG-BATCH      │ Created 1768 new groups
56ms  │ AGG-FINAL      │ After aggregation: total_groups=1768
      │                │
... (more partial batches) ...
      │                │
80ms  │ AGG-FINAL      │ Hit soft group limit: groups=10
81ms  │ MEMORY         │ BEFORE emit(All): total=156KB, groups=10
82ms  │ MEMORY         │ AFTER emit: emitted_rows=10, freed=154KB
83ms  │ AGG-OUTPUT     │ ✓ Returning batch to upstream: rows=10
      │                │
84ms  │ TOPK-INSERT    │ ▼ Receiving batch: rows=10, heap=0/10
85ms  │ TOPK-INSERT    │ Processed: added=10, rejected=0
86ms  │ TOPK-INSERT    │ ✓ Batch processed: heap=10/10
      │                │
90ms  │ TOPK-EMIT      │ Starting final emission: heap_size=10
91ms  │ TOPK-EMIT      │ ✓ Emission complete: 1 output batches
```

## State Transitions

### GroupedHashAggregateStream States

```
┌──────────────────┐
│  ReadingInput    │◄─────────────────┐
└────────┬─────────┘                  │
         │                             │
         │ emit triggered              │
         ▼                             │
┌──────────────────┐                  │
│ ProducingOutput  │                  │
└────────┬─────────┘                  │
         │                             │
         │ batch fully output          │
         │ & !input_done               │
         └─────────────────────────────┘
         │
         │ batch fully output
         │ & input_done
         ▼
┌──────────────────┐
│      Done        │
└──────────────────┘
```

### TopK States

```
┌──────────────────┐
│  Accumulating    │
│  (insert_batch)  │
└────────┬─────────┘
         │
         │ All batches processed
         ▼
┌──────────────────┐
│   Emitting       │
│   (emit)         │
└────────┬─────────┘
         │
         │ Stream consumed
         ▼
┌──────────────────┐
│      Done        │
└──────────────────┘
```

## Key Takeaways

1. **Partial Aggregation** accumulates groups from raw input
2. **Final Aggregation** merges partial results and applies soft limit
3. **Soft Limit** (from LIMIT clause) triggers early emission in Final mode
4. **TopK** maintains a bounded heap of top K rows
5. **Memory** is freed when groups are emitted
6. **Batches flow** from Partial → Final → TopK → Output
7. **EmitTo** determines what groups to emit (All, First(n), NextBlock)
