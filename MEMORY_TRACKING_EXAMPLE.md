# Memory Tracking for RecordBatchStore

## Add logging to the insert method:

```rust
pub fn insert(&mut self, entry: RecordBatchEntry) {
    // Log BEFORE insert
    let size_before = self.size();
    let batches_count_before = self.batches.len();
    
    // uses of 0 means that none of the rows in the batch were stored in the topk
    if entry.uses > 0 {
        let entry_size = entry.batch.get_array_memory_size();
        
        log::info!(
            "[TOPK-MEMORY] BEFORE insert: total={}KB, batches_count={}, batches_size={}KB",
            size_before / 1024,
            batches_count_before,
            self.batches_size / 1024
        );
        
        self.batches_size += entry_size;
        self.batches.insert(entry.id, entry);
        
        // Log AFTER insert
        let size_after = self.size();
        log::info!(
            "[TOPK-MEMORY] AFTER insert: total={}KB, batches_count={}, batches_size={}KB, entry_added={}KB, memory_increase={}KB",
            size_after / 1024,
            self.batches.len(),
            self.batches_size / 1024,
            entry_size / 1024,
            (size_after - size_before) / 1024
        );
    }
}
```

## Add logging to the size method (optional):

```rust
pub fn size(&self) -> usize {
    let struct_size = std::mem::size_of::<Self>();
    let capacity_size = self.batches.capacity()
        * (std::mem::size_of::<u32>() + std::mem::size_of::<RecordBatchEntry>());
    let total = struct_size + capacity_size + self.batches_size;
    
    log::debug!(
        "[TOPK-MEMORY] size() called: struct={}KB, capacity={}KB, batches={}KB, total={}KB",
        struct_size / 1024,
        capacity_size / 1024,
        self.batches_size / 1024,
        total / 1024
    );
    
    total
}
```

## Enable logging:

```bash
# Enable info logs for topk module
export RUST_LOG=datafusion_physical_plan::aggregates::topk=info

# Or enable for all aggregates
export RUST_LOG=datafusion_physical_plan::aggregates=info
```

## Expected output:

```
[TOPK-MEMORY] BEFORE insert: total=42KB, batches_count=5, batches_size=40KB
[TOPK-MEMORY] AFTER insert: total=50KB, batches_count=6, batches_size=48KB, entry_added=8KB, memory_increase=8KB
```

This will help you track:
- Total memory used by RecordBatchStore
- Number of batches stored
- Size of batch data
- Memory increase per insert
