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

//! End-to-end coverage for compact integer IN-list pruning.
//!
//! Modeled after [`super::string_in_list_pruning`]. Verifies that an Int64
//! IN list with >20 entries (when `max_in_list_size` is raised) triggers
//! `IntegerInListPruningExpr` for both row-group and page-index pruning,
//! including correct metrics assertions.

use std::sync::Arc;

use arrow::array::Int64Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use arrow::util::pretty::pretty_format_batches;
use datafusion::physical_plan::{collect, displayable};
use datafusion::prelude::{ParquetReadOptions, SessionConfig, SessionContext};
use datafusion_common::assert_batches_eq;
use datafusion_physical_plan::metrics::{MetricValue, MetricsSet};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::{EnabledStatistics, WriterProperties};
use tempfile::NamedTempFile;

use super::utils::MetricsFinder;

/// Rows per unit (row group or page).
const ROWS_PER_UNIT: usize = 16;
/// Number of units (row groups or pages within one row group).
const UNITS: usize = 4;
const TOTAL_ROWS: usize = ROWS_PER_UNIT * UNITS;
/// Units 0 and 2 contain values in the IN list; units 1 and 3 do not.
const MATCHING_ROWS: usize = ROWS_PER_UNIT * 2;

/// Write a Parquet file with 4 units of Int64 data:
///   Unit 0: all values = 0 (matches IN list member 0)
///   Unit 1: all values = 5 (falls in gap between IN list members 0 and 10)
///   Unit 2: all values = 100 (matches IN list member 100)
///   Unit 3: all values = 9999 (above all IN list members)
///
/// When `page_pruning` is true, a single row group with 4 pages; otherwise 4
/// row groups with a single page each.
fn make_file(page_pruning: bool) -> NamedTempFile {
    let mut file = tempfile::Builder::new()
        .prefix("integer_in_list_pruning")
        .suffix(".parquet")
        .tempfile()
        .unwrap();
    let schema = Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Int64,
        false,
    )]));
    let unit_values: [i64; 4] = [0, 5, 100, 9999];
    let values: Vec<i64> = unit_values
        .iter()
        .flat_map(|v| std::iter::repeat_n(*v, ROWS_PER_UNIT))
        .collect();
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![Arc::new(Int64Array::from(values))],
    )
    .unwrap();
    let rows_per_group = if page_pruning {
        TOTAL_ROWS
    } else {
        ROWS_PER_UNIT
    };
    let properties = WriterProperties::builder()
        .set_max_row_group_row_count(Some(rows_per_group))
        .set_data_page_row_count_limit(ROWS_PER_UNIT)
        .set_write_batch_size(ROWS_PER_UNIT)
        .set_dictionary_enabled(false)
        .set_bloom_filter_enabled(false)
        .set_statistics_enabled(EnabledStatistics::Page)
        .build();
    let mut writer = ArrowWriter::try_new(&mut file, schema, Some(properties)).unwrap();
    writer.write(&batch).unwrap();
    let metadata = writer.close().unwrap();
    assert_eq!(metadata.num_row_groups(), TOTAL_ROWS / rows_per_group);
    let offsets = metadata.offset_index().unwrap();
    for row_group in offsets {
        assert_eq!(
            row_group[0].page_locations().len(),
            rows_per_group / ROWS_PER_UNIT
        );
    }
    file
}

struct ScanOutput {
    batches: Vec<RecordBatch>,
    plan: String,
    metrics: MetricsSet,
}

impl ScanOutput {
    fn counter(&self, name: &str) -> usize {
        self.metrics
            .sum(|metric| metric.value().name() == name)
            .unwrap_or_else(|| panic!("missing {name}: {}", self.metrics))
            .as_usize()
    }

    fn pruned(&self, name: &str) -> usize {
        let value = self
            .metrics
            .sum(|metric| metric.value().name() == name)
            .unwrap_or_else(|| panic!("missing {name}: {}", self.metrics));
        let MetricValue::PruningMetrics {
            pruning_metrics, ..
        } = value
        else {
            panic!("expected pruning metric {name}: {}", self.metrics);
        };
        pruning_metrics.pruned()
    }

    fn assert_results(&self) {
        // Only values 0 and 100 are in the IN list and present in the data.
        assert_batches_eq!(
            [
                "+-------+----+",
                "| value | n  |",
                "+-------+----+",
                "| 0     | 16 |",
                "| 100   | 16 |",
                "+-------+----+",
            ],
            &self.batches
        );
        assert_eq!(self.counter("predicate_evaluation_errors"), 0);
        assert_eq!(self.counter("pushdown_rows_pruned"), 0);
        assert_eq!(self.pruned("row_groups_pruned_bloom_filter"), 0);
    }
}

async fn scan(
    file: &NamedTempFile,
    list_size: usize,
    max_in_list_size: Option<usize>,
    page_pruning: bool,
) -> ScanOutput {
    let mut config = SessionConfig::new()
        .with_target_partitions(1)
        .with_parquet_bloom_filter_pruning(false)
        .with_parquet_page_index_pruning(page_pruning);
    config.options_mut().execution.parquet.pushdown_filters = false;
    if let Some(max_in_list_size) = max_in_list_size {
        config.options_mut().execution.parquet.max_in_list_size = max_in_list_size;
    }
    let ctx = SessionContext::new_with_config(config);
    ctx.register_parquet(
        "t",
        file.path().to_str().unwrap(),
        ParquetReadOptions::default(),
    )
    .await
    .unwrap();
    // Build an IN list with `list_size` entries. Members are spaced 10 apart:
    // 0, 10, 20, ..., (list_size-1)*10. This ensures value 0 and 100 are in
    // the list (for sizes >= 11), while 5 and 9999 are NOT.
    let values = (0..list_size)
        .map(|index| (index * 10).to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT value, count(*) AS n FROM t \
         WHERE value IN ({values}) GROUP BY value ORDER BY value"
    );
    let plan = ctx
        .sql(&sql)
        .await
        .unwrap()
        .create_physical_plan()
        .await
        .unwrap();
    let plan_text = displayable(plan.as_ref()).indent(true).to_string();
    let batches = collect(Arc::clone(&plan), ctx.task_ctx()).await.unwrap();
    let metrics = MetricsFinder::find_metrics(plan.as_ref()).unwrap();
    ScanOutput {
        batches,
        plan: plan_text,
        metrics,
    }
}

/// Core check: for each list size, verify pruning metrics and result
/// correctness under both the unpruned control and the compact path.
async fn check_integer_in_list_pruning(page_pruning: bool) {
    let file = make_file(page_pruning);
    for list_size in [21, 256, 1024] {
        // A zero cap provides a result-equivalence control that cannot use
        // min/max IN-list pruning at either granularity.
        let unpruned = scan(&file, list_size, Some(0), page_pruning).await;
        unpruned.assert_results();
        assert!(!unpruned.plan.contains("INT_IN_SET_INTERSECTS"));
        assert_eq!(unpruned.pruned("row_groups_pruned_statistics"), 0);
        assert_eq!(unpruned.pruned("page_index_rows_pruned"), 0);
        assert_eq!(unpruned.counter("output_rows"), TOTAL_ROWS);

        let output = scan(&file, list_size, Some(list_size), page_pruning).await;
        output.assert_results();
        // Result equivalence with the unpruned control.
        assert_eq!(
            pretty_format_batches(&output.batches).unwrap().to_string(),
            pretty_format_batches(&unpruned.batches)
                .unwrap()
                .to_string()
        );
        // The compact path must be used (list_size > 20).
        assert!(
            output.plan.contains("INT_IN_SET_INTERSECTS"),
            "list_size={list_size}, plan={}",
            output.plan
        );
        // Units 1 (value=5, gap) and 3 (value=9999, above domain) are pruned.
        assert_eq!(
            output.pruned("row_groups_pruned_statistics"),
            if page_pruning { 0 } else { 2 },
            "list_size={list_size}, metrics={}",
            output.metrics
        );
        assert_eq!(
            output.pruned("page_index_rows_pruned"),
            if page_pruning { MATCHING_ROWS } else { 0 },
            "list_size={list_size}, metrics={}",
            output.metrics
        );
        assert_eq!(output.counter("output_rows"), MATCHING_ROWS);
    }

    // The default max_in_list_size is 20: a 21-item list must NOT trigger the
    // compact path unless the cap is explicitly raised.
    let default = scan(&file, 21, None, page_pruning).await;
    default.assert_results();
    assert!(!default.plan.contains("INT_IN_SET_INTERSECTS"));
    assert_eq!(default.pruned("row_groups_pruned_statistics"), 0);
    assert_eq!(default.pruned("page_index_rows_pruned"), 0);
    assert_eq!(default.counter("output_rows"), TOTAL_ROWS);
}

#[tokio::test]
async fn integer_in_list_row_group_pruning() {
    check_integer_in_list_pruning(false).await;
}

#[tokio::test]
async fn integer_in_list_page_pruning() {
    check_integer_in_list_pruning(true).await;
}

/// Verify that the N=20 threshold is a control: compact path is NOT used.
/// This documents that N=20 is the boundary where `compact=false`; N=21 is
/// the first treatment where the compact `IntegerInListPruningExpr` fires.
#[tokio::test]
async fn integer_in_list_threshold_boundary() {
    let file = make_file(false);

    // N=20 at cap=20: falls into the per-value OR expansion path, not compact.
    let at_threshold = scan(&file, 20, Some(20), false).await;
    assert!(
        !at_threshold.plan.contains("INT_IN_SET_INTERSECTS"),
        "N=20 must NOT use compact path (it equals MAX_IN_LIST_SIZE), plan={}",
        at_threshold.plan
    );

    // N=21 at cap=21: first size that triggers the compact path.
    let above_threshold = scan(&file, 21, Some(21), false).await;
    assert!(
        above_threshold.plan.contains("INT_IN_SET_INTERSECTS"),
        "N=21 must use compact path, plan={}",
        above_threshold.plan
    );
}
