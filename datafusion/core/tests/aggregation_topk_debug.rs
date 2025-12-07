// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Test to understand GroupedHashAggregateStream and TopK interaction
//! 
//! This test helps debug how groups are stored, when they are emitted,
//! and how memory/objects pass between aggregation and topk layers.
//!
//! ## What to look for when debugging:
//! 
//! 1. **Group Storage**: Watch for log messages about group_values being created/updated
//! 2. **Group Emission**: Look for when groups are emitted from the hash table
//! 3. **Memory Tracking**: Monitor memory reservation changes
//! 4. **TopK Interaction**: See when batches flow from aggregation to TopK
//! 5. **Batch Sizes**: Track the number of rows in each batch at each stage
//!
//! ## Running this test:
//! 
//! ```bash
//! RUST_LOG=debug cargo test --test aggregation_topk_debug -- --nocapture
//! ```
//!
//! Or for more focused output:
//! ```bash
//! RUST_LOG=datafusion_physical_plan=debug cargo test --test aggregation_topk_debug -- --nocapture
//! ```

use datafusion::prelude::*;
use datafusion_common::Result;
use std::sync::Arc;

#[tokio::test]
async fn test_aggregation_topk_interaction() -> Result<()> {
    // Initialize logging to see debug output
    // Use Info level by default for cleaner output, set to Debug for more detail
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(log::LevelFilter::Info);
    
    let _ = env_logger::builder()
        .filter_level(log_level)
        .is_test(true)
        .try_init();

    println!("\n{'='*80}");
    println!("AGGREGATION + TOPK INTERACTION DEBUG TEST");
    println!("{'='*80}\n");

    // Create a session context with custom configuration
    let config = SessionConfig::new()
        .with_batch_size(8192)  // Standard batch size
        .with_target_partitions(1); // Single partition for easier debugging
    
    let ctx = SessionContext::new_with_config(config);

    // Register the partitioned clickbench data
    let data_path = "/Users/abandeji/Public/work-dump/clickbench_data/partitioned";
    
    println!("=== STEP 1: REGISTERING TABLE ===");
    println!("Data path: {}", data_path);
    
    ctx.register_parquet(
        "hits",
        data_path,
        ParquetReadOptions::default(),
    )
    .await?;
    
    println!("✓ Table registered successfully\n");

    println!("=== STEP 2: BUILDING QUERY ===");
    // The query from the user's request
    let sql = r#"
        SELECT "WatchID", 
               MIN("ResolutionWidth") as min_width, 
               MAX("ResolutionWidth") as max_width, 
               SUM("IsRefresh") as sum_refresh
        FROM hits 
        GROUP BY "WatchID" 
        ORDER BY "WatchID" DESC 
        LIMIT 10
    "#;
    
    println!("Query: {}", sql);

    let df = ctx.sql(sql).await?;
    
    println!("\n=== STEP 3: LOGICAL PLAN ===");
    println!("{}", df.clone().logical_plan());
    
    println!("\n=== STEP 4: OPTIMIZED LOGICAL PLAN ===");
    println!("{}", df.clone().into_optimized_plan()?);
    
    println!("\n=== STEP 5: PHYSICAL PLAN ===");
    println!("{}", df.clone().create_physical_plan().await?);
    
    println!("\n{'='*80}");
    println!("EXECUTING QUERY - WATCH FOR:");
    println!("  - GroupedHashAggregateStream creating groups");
    println!("  - Memory reservations and updates");
    println!("  - Batch emissions from aggregation");
    println!("  - TopK receiving and processing batches");
    println!("  - Final result assembly");
    println!("{'='*80}\n");
    
    let start = std::time::Instant::now();
    let results = df.collect().await?;
    let duration = start.elapsed();

    println!("\n{'='*80}");
    println!("QUERY COMPLETED in {:?}", duration);
    println!("{'='*80}\n");

    println!("=== STEP 6: RESULTS ===");
    println!("Total batches returned: {}", results.len());
    
    for (i, batch) in results.iter().enumerate() {
        println!("\nBatch #{}: {} rows, {} columns", 
                 i + 1, 
                 batch.num_rows(), 
                 batch.num_columns());
        
        // Print schema
        println!("Schema: {:?}", batch.schema());
        
        // Print first few rows for verification
        if batch.num_rows() > 0 {
            println!("\nSample data:");
            for row_idx in 0..batch.num_rows().min(5) {
                print!("  Row {}: ", row_idx);
                for col_idx in 0..batch.num_columns() {
                    let column = batch.column(col_idx);
                    print!("{:?} ", column.slice(row_idx, 1));
                }
                println!();
            }
        }
    }

    println!("\n{'='*80}");
    println!("TEST COMPLETED SUCCESSFULLY");
    println!("{'='*80}\n");

    Ok(())
}
