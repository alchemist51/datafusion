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

//! Compare compact integer IN-list pruning with per-value min/max expansion.
//!
//! Both cases raise `max_in_list_size` to the domain size. The `in_list` case
//! triggers the new `IntegerInListPruningExpr` for sizes > 20. The explicit
//! `expanded_or` is a balanced tree of equalities that produces per-value
//! statistics checks. Half of the statistics intervals hit a domain member and
//! half fall in a sparse gap.
//!
//! ## Domain sizes and the threshold boundary
//!
//! - **N=20**: This is the `MAX_IN_LIST_SIZE` default. At this size the compact
//!   path is NOT used (`compact=false`); the IN list takes the per-value OR
//!   expansion path. This case serves as the threshold-control baseline to
//!   confirm that the boundary is respected.
//! - **N=21**: The first "treatment" size — one above the threshold — where
//!   `IntegerInListPruningExpr` fires. Comparing N=20 vs N=21 isolates the
//!   compact path's incremental construction and evaluation cost.
//! - **N=256, N=1024**: Stress the compact path at scale to show O(log N)
//!   evaluation cost vs O(N) for the expanded OR tree.
//!
//! Run with `cargo bench -p datafusion-pruning --bench integer_in_list_pruning`.

use std::collections::HashSet;
use std::hint::black_box;
use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanArray, Int64Array, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use datafusion_common::{Column, ScalarValue};
use datafusion_expr_common::operator::Operator;
use datafusion_physical_expr::PhysicalExprRef;
use datafusion_physical_expr::expressions::{BinaryExpr, col, in_list, lit};
use datafusion_pruning::{PruningPredicate, PruningPredicateBuilder, PruningStatistics};

const DOMAIN_SIZES: [usize; 4] = [20, 21, 256, 1024];
const CONTAINERS: usize = 4096;

fn balanced_or(expressions: &[PhysicalExprRef]) -> PhysicalExprRef {
    if expressions.len() == 1 {
        return Arc::clone(&expressions[0]);
    }
    let middle = expressions.len() / 2;
    Arc::new(BinaryExpr::new(
        balanced_or(&expressions[..middle]),
        Operator::Or,
        balanced_or(&expressions[middle..]),
    ))
}

fn build_predicate(
    expression: &PhysicalExprRef,
    schema: &SchemaRef,
    max_in_list_size: usize,
) -> PruningPredicate {
    PruningPredicateBuilder::new()
        .with_file_schema(Arc::clone(schema))
        .with_max_in_list_size(max_in_list_size)
        .try_build(Arc::clone(expression))
        .unwrap()
}

struct IntervalStatistics {
    min: ArrayRef,
    max: ArrayRef,
    null_counts: ArrayRef,
    row_counts: ArrayRef,
}

impl IntervalStatistics {
    fn new(domain_size: usize) -> Self {
        // Each domain value is spaced 10 apart (value * 10).
        // Even containers hit a domain member; odd containers fall in gaps.
        let min = Int64Array::from_iter_values((0..CONTAINERS).map(|index| {
            let start = (index / 2 % domain_size) * 10;
            (start + if index % 2 == 0 { 0 } else { 3 }) as i64
        }));
        let max = Int64Array::from_iter_values((0..CONTAINERS).map(|index| {
            let start = (index / 2 % domain_size) * 10;
            (start + if index % 2 == 0 { 0 } else { 7 }) as i64
        }));
        Self {
            min: Arc::new(min),
            max: Arc::new(max),
            null_counts: Arc::new(UInt64Array::from(vec![0; CONTAINERS])),
            row_counts: Arc::new(UInt64Array::from(vec![128; CONTAINERS])),
        }
    }
}

impl PruningStatistics for IntervalStatistics {
    fn min_values(&self, column: &Column) -> Option<ArrayRef> {
        (column.name == "value").then(|| Arc::clone(&self.min))
    }

    fn max_values(&self, column: &Column) -> Option<ArrayRef> {
        (column.name == "value").then(|| Arc::clone(&self.max))
    }

    fn num_containers(&self) -> usize {
        CONTAINERS
    }

    fn null_counts(&self, column: &Column) -> Option<ArrayRef> {
        (column.name == "value").then(|| Arc::clone(&self.null_counts))
    }

    fn row_counts(&self) -> Option<ArrayRef> {
        Some(Arc::clone(&self.row_counts))
    }

    fn contained(
        &self,
        _column: &Column,
        _values: &HashSet<ScalarValue>,
    ) -> Option<BooleanArray> {
        None
    }
}

struct BenchmarkCase {
    size: usize,
    schema: SchemaRef,
    in_list: PhysicalExprRef,
    expanded_or: PhysicalExprRef,
    in_list_predicate: PruningPredicate,
    expanded_or_predicate: PruningPredicate,
    statistics: IntervalStatistics,
}

impl BenchmarkCase {
    fn new(size: usize) -> Self {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "value",
            DataType::Int64,
            false,
        )]));
        let column = col("value", &schema).unwrap();
        let values = (0..size)
            .map(|index| lit(ScalarValue::Int64(Some((index * 10) as i64))))
            .collect::<Vec<_>>();
        let in_list_expr =
            in_list(Arc::clone(&column), values.clone(), &false, &schema).unwrap();
        let equalities = values
            .into_iter()
            .map(|value| {
                Arc::new(BinaryExpr::new(Arc::clone(&column), Operator::Eq, value))
                    as PhysicalExprRef
            })
            .collect::<Vec<_>>();
        let expanded_or = balanced_or(&equalities);
        let in_list_predicate = build_predicate(&in_list_expr, &schema, size);
        let expanded_or_predicate = build_predicate(&expanded_or, &schema, size);
        eprintln!(
            "integer_in_list_pruning: {size} values, compact={}",
            in_list_predicate
                .predicate_expr()
                .to_string()
                .contains("INT_IN_SET_INTERSECTS")
        );
        let statistics = IntervalStatistics::new(size);

        // Verify both paths produce the same results.
        let expected = (0..CONTAINERS)
            .map(|index| index % 2 == 0)
            .collect::<Vec<_>>();
        assert_eq!(in_list_predicate.prune(&statistics).unwrap(), expected);
        assert_eq!(expanded_or_predicate.prune(&statistics).unwrap(), expected);

        Self {
            size,
            schema,
            in_list: in_list_expr,
            expanded_or,
            in_list_predicate,
            expanded_or_predicate,
            statistics,
        }
    }
}

fn criterion_benchmark(criterion: &mut Criterion) {
    let cases = DOMAIN_SIZES.map(BenchmarkCase::new);
    let mut construction = criterion.benchmark_group("integer_in_list_pruning/construct");
    for case in &cases {
        construction.throughput(Throughput::Elements(case.size as u64));
        for (name, expression) in [
            ("in_list", &case.in_list),
            ("expanded_or", &case.expanded_or),
        ] {
            construction.bench_with_input(
                BenchmarkId::new(name, case.size),
                expression,
                |bencher, expression| {
                    bencher.iter(|| {
                        black_box(build_predicate(
                            black_box(expression),
                            &case.schema,
                            case.size,
                        ))
                    });
                },
            );
        }
    }
    construction.finish();

    let mut evaluation = criterion.benchmark_group("integer_in_list_pruning/evaluate");
    evaluation.throughput(Throughput::Elements(CONTAINERS as u64));
    for case in &cases {
        for (name, predicate) in [
            ("in_list", &case.in_list_predicate),
            ("expanded_or", &case.expanded_or_predicate),
        ] {
            evaluation.bench_with_input(
                BenchmarkId::new(name, case.size),
                predicate,
                |bencher, predicate| {
                    bencher.iter(|| {
                        black_box(predicate.prune(black_box(&case.statistics)).unwrap())
                    });
                },
            );
        }
    }
    evaluation.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
