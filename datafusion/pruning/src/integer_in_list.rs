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

//! Compact IN-list pruning for ordered integer-like types.
//!
//! Analogous to [`crate::string_in_list::StringInListPruningExpr`] but operates
//! on signed/unsigned integers and date/time/timestamp types. Values are
//! normalized to `i128` for a single sorted domain with O(log N) binary search
//! against min/max statistics intervals.
//!
//! ## Excluded types
//!
//! **Float types (`Float16`, `Float32`, `Float64`)** are excluded because IEEE
//! 754 NaN violates total ordering: NaN != NaN, and NaN is neither less than
//! nor greater than any value. A domain containing NaN cannot produce correct
//! binary-search results against min/max statistics, which may themselves be
//! NaN (Parquet propagates NaN into statistics). Rather than introducing a
//! special NaN-aware comparator with edge-case risk, floats are left to the
//! existing per-value expansion path.
//!
//! **Decimal types (`Decimal128`, `Decimal256`)** are excluded because:
//! 1. `i128` can represent `Decimal128` raw values but NOT `Decimal256` (which
//!    requires `i256`). Supporting only one decimal width introduces an
//!    inconsistency.
//! 2. Scale normalization: comparing decimals with different (precision, scale)
//!    requires rescaling. The planner's type coercion pass normalizes IN-list
//!    entries to the column's type, but any mismatch that slips through would
//!    produce silently wrong pruning decisions.
//! 3. Decimal IN lists large enough to exceed `MAX_IN_LIST_SIZE` are rare in
//!    practice — the per-value expansion path is adequate.
//!
//! **Timestamp timezone awareness:** Timestamps with or without timezone are
//! supported. Arrow stores all timestamps as epoch-based integer offsets
//! regardless of the timezone annotation; the physical representation is
//! identical. Planner type coercion ensures that literal values in the IN list
//! are cast to the column's declared type (unit + timezone) before reaching
//! the pruning layer, so comparing raw i64 epoch values is semantically
//! correct.

use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use arrow::array::{Array, AsArray, BooleanArray};
use arrow::datatypes::{
    DataType, Date32Type, Date64Type, Int8Type, Int16Type, Int32Type, Int64Type, Schema,
    Time32MillisecondType, Time32SecondType, Time64MicrosecondType, Time64NanosecondType,
    TimestampMicrosecondType, TimestampMillisecondType, TimestampNanosecondType,
    TimestampSecondType, UInt8Type, UInt16Type, UInt32Type, UInt64Type,
};
use datafusion_common::{Result, ScalarValue, assert_eq_or_internal_err};
use datafusion_physical_expr::{PhysicalExpr, PhysicalExprRef};
use datafusion_physical_plan::ColumnarValue;

use arrow::record_batch::RecordBatch;

/// Tests whether a sorted integer domain intersects an inclusive statistics interval.
///
/// [`PhysicalExpr::evaluate`] returns one nullable Boolean per min/max interval:
/// * `true`: the interval intersects the domain, so matching rows may exist.
/// * `false`: the available bounds prove the interval disjoint from the domain.
/// * `NULL`: incomplete, invalid, or unusable bounds prevent a safe decision.
///
/// This expression is used only for pruning; the original IN remains the row filter.
#[derive(Debug, Eq)]
pub(crate) struct IntegerInListPruningExpr {
    min: PhysicalExprRef,
    max: PhysicalExprRef,
    /// Sorted, deduplicated i128 values representing the IN list domain.
    values: Arc<[i128]>,
    /// The original data type for display purposes.
    data_type: DataType,
}

impl IntegerInListPruningExpr {
    pub(crate) fn new(
        min: PhysicalExprRef,
        max: PhysicalExprRef,
        mut values: Vec<i128>,
        data_type: DataType,
    ) -> Self {
        // Invariant: the domain must be non-empty. An empty IN list is
        // semantically always-false and should be folded by the optimizer
        // before reaching the pruning layer.
        debug_assert!(
            !values.is_empty(),
            "IntegerInListPruningExpr requires a non-empty domain"
        );
        values.sort_unstable();
        values.dedup();
        Self {
            min,
            max,
            values: values.into(),
            data_type,
        }
    }
}

impl PartialEq for IntegerInListPruningExpr {
    fn eq(&self, other: &Self) -> bool {
        self.min.eq(&other.min)
            && self.max.eq(&other.max)
            && self.values == other.values
            && self.data_type == other.data_type
    }
}

impl Hash for IntegerInListPruningExpr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.min.hash(state);
        self.max.hash(state);
        self.values.hash(state);
        self.data_type.hash(state);
    }
}

impl Display for IntegerInListPruningExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "INT_IN_SET_INTERSECTS({}, {}, {} values, {:?})",
            self.min,
            self.max,
            self.values.len(),
            self.data_type
        )
    }
}

/// Returns `true` if the data type is a supported ordered integer-like type.
pub(crate) fn is_supported_int_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Date32
            | DataType::Date64
            | DataType::Time32(_)
            | DataType::Time64(_)
            | DataType::Timestamp(_, _)
    )
}

/// Extract an i128 from a ScalarValue for a supported integer-like type.
pub(crate) fn scalar_to_i128(value: &ScalarValue) -> Option<i128> {
    match value {
        ScalarValue::Int8(Some(v)) => Some(*v as i128),
        ScalarValue::Int16(Some(v)) => Some(*v as i128),
        ScalarValue::Int32(Some(v)) => Some(*v as i128),
        ScalarValue::Int64(Some(v)) => Some(*v as i128),
        ScalarValue::UInt8(Some(v)) => Some(*v as i128),
        ScalarValue::UInt16(Some(v)) => Some(*v as i128),
        ScalarValue::UInt32(Some(v)) => Some(*v as i128),
        ScalarValue::UInt64(Some(v)) => Some(*v as i128),
        ScalarValue::Date32(Some(v)) => Some(*v as i128),
        ScalarValue::Date64(Some(v)) => Some(*v as i128),
        ScalarValue::Time32Second(Some(v)) => Some(*v as i128),
        ScalarValue::Time32Millisecond(Some(v)) => Some(*v as i128),
        ScalarValue::Time64Microsecond(Some(v)) => Some(*v as i128),
        ScalarValue::Time64Nanosecond(Some(v)) => Some(*v as i128),
        ScalarValue::TimestampSecond(Some(v), _) => Some(*v as i128),
        ScalarValue::TimestampMillisecond(Some(v), _) => Some(*v as i128),
        ScalarValue::TimestampMicrosecond(Some(v), _) => Some(*v as i128),
        ScalarValue::TimestampNanosecond(Some(v), _) => Some(*v as i128),
        _ => None,
    }
}

/// Extract i128 from an Arrow array element at the given index.
fn array_value_to_i128(array: &dyn Array, index: usize) -> Option<i128> {
    if array.is_null(index) {
        return None;
    }
    match array.data_type() {
        DataType::Int8 => Some(array.as_primitive::<Int8Type>().value(index) as i128),
        DataType::Int16 => Some(array.as_primitive::<Int16Type>().value(index) as i128),
        DataType::Int32 => Some(array.as_primitive::<Int32Type>().value(index) as i128),
        DataType::Int64 => Some(array.as_primitive::<Int64Type>().value(index) as i128),
        DataType::UInt8 => Some(array.as_primitive::<UInt8Type>().value(index) as i128),
        DataType::UInt16 => Some(array.as_primitive::<UInt16Type>().value(index) as i128),
        DataType::UInt32 => Some(array.as_primitive::<UInt32Type>().value(index) as i128),
        DataType::UInt64 => Some(array.as_primitive::<UInt64Type>().value(index) as i128),
        DataType::Date32 => Some(array.as_primitive::<Date32Type>().value(index) as i128),
        DataType::Date64 => Some(array.as_primitive::<Date64Type>().value(index) as i128),
        DataType::Time32(arrow::datatypes::TimeUnit::Second) => {
            Some(array.as_primitive::<Time32SecondType>().value(index) as i128)
        }
        DataType::Time32(arrow::datatypes::TimeUnit::Millisecond) => {
            Some(array.as_primitive::<Time32MillisecondType>().value(index) as i128)
        }
        DataType::Time64(arrow::datatypes::TimeUnit::Microsecond) => {
            Some(array.as_primitive::<Time64MicrosecondType>().value(index) as i128)
        }
        DataType::Time64(arrow::datatypes::TimeUnit::Nanosecond) => {
            Some(array.as_primitive::<Time64NanosecondType>().value(index) as i128)
        }
        DataType::Timestamp(arrow::datatypes::TimeUnit::Second, _) => {
            Some(array.as_primitive::<TimestampSecondType>().value(index) as i128)
        }
        DataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, _) => Some(
            array
                .as_primitive::<TimestampMillisecondType>()
                .value(index) as i128,
        ),
        DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, _) => Some(
            array
                .as_primitive::<TimestampMicrosecondType>()
                .value(index) as i128,
        ),
        DataType::Timestamp(arrow::datatypes::TimeUnit::Nanosecond, _) => {
            Some(array.as_primitive::<TimestampNanosecondType>().value(index) as i128)
        }
        // Dictionary: unwrap to value array
        DataType::Dictionary(_, _) => {
            let dict = array.as_any_dictionary();
            let key_index = dict.normalized_keys()[index];
            array_value_to_i128(dict.values().as_ref(), key_index)
        }
        _ => None,
    }
}

impl PhysicalExpr for IntegerInListPruningExpr {
    fn data_type(&self, _input_schema: &Schema) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn nullable(&self, _input_schema: &Schema) -> Result<bool> {
        Ok(true)
    }

    fn evaluate(&self, batch: &RecordBatch) -> Result<ColumnarValue> {
        let min_arr = self.min.evaluate(batch)?.into_array(batch.num_rows())?;
        let max_arr = self.max.evaluate(batch)?.into_array(batch.num_rows())?;

        let matches: BooleanArray = (0..batch.num_rows())
            .map(|i| {
                let min_val = array_value_to_i128(min_arr.as_ref(), i);
                let max_val = array_value_to_i128(max_arr.as_ref(), i);

                match (min_val, max_val) {
                    (Some(min), Some(max)) => {
                        if min > max {
                            return None;
                        }
                        // Binary search: find the first value >= min
                        let index = self.values.partition_point(|v| *v < min);
                        // Check if this value is <= max
                        Some(self.values.get(index).is_some_and(|v| *v <= max))
                    }
                    // A missing bound: exclude only when entire domain is
                    // beyond the known bound.
                    (Some(min), None) if self.values.last().is_some_and(|v| *v < min) => {
                        Some(false)
                    }
                    (None, Some(max))
                        if self.values.first().is_some_and(|v| *v > max) =>
                    {
                        Some(false)
                    }
                    _ => None,
                }
            })
            .collect();

        Ok(ColumnarValue::Array(Arc::new(matches)))
    }

    fn children(&self) -> Vec<&PhysicalExprRef> {
        vec![&self.min, &self.max]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<PhysicalExprRef>,
    ) -> Result<PhysicalExprRef> {
        assert_eq_or_internal_err!(children.len(), 2);
        Ok(Arc::new(Self {
            min: Arc::clone(&children[0]),
            max: Arc::clone(&children[1]),
            values: Arc::clone(&self.values),
            data_type: self.data_type.clone(),
        }))
    }

    fn fmt_sql(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{
        ArrayRef, Date32Array, DictionaryArray, Int8Array, Int64Array,
        TimestampMicrosecondArray, UInt64Array,
    };
    use arrow::datatypes::{Field, Int32Type as DictKeyType};
    use datafusion_physical_expr::expressions::Column;

    fn make_schema() -> Schema {
        Schema::new(vec![
            Field::new("min", DataType::Int64, true),
            Field::new("max", DataType::Int64, true),
        ])
    }

    fn make_expr(values: Vec<i128>, data_type: DataType) -> IntegerInListPruningExpr {
        let schema = make_schema();
        let min: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        IntegerInListPruningExpr::new(min, max, values, data_type)
    }

    fn eval_with_int64(
        expr: &IntegerInListPruningExpr,
        mins: Vec<Option<i64>>,
        maxs: Vec<Option<i64>>,
    ) -> Vec<Option<bool>> {
        let schema = Arc::new(make_schema());
        let min_arr: ArrayRef = Arc::new(Int64Array::from(mins));
        let max_arr: ArrayRef = Arc::new(Int64Array::from(maxs));
        let batch = RecordBatch::try_new(schema, vec![min_arr, max_arr]).unwrap();
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(batch.num_rows()).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        (0..bool_arr.len())
            .map(|i| {
                if bool_arr.is_null(i) {
                    None
                } else {
                    Some(bool_arr.value(i))
                }
            })
            .collect()
    }

    #[test]
    fn test_exact_hit() {
        // Domain: [10, 20, 30]. Interval [20, 20] => exact hit.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(20)], vec![Some(20)]);
        assert_eq!(result, vec![Some(true)]);
    }

    #[test]
    fn test_range_hit() {
        // Domain: [10, 20, 30]. Interval [15, 25] => contains 20.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(15)], vec![Some(25)]);
        assert_eq!(result, vec![Some(true)]);
    }

    #[test]
    fn test_gap_miss() {
        // Domain: [10, 20, 30]. Interval [11, 19] => falls in gap.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(11)], vec![Some(19)]);
        assert_eq!(result, vec![Some(false)]);
    }

    #[test]
    fn test_below_domain() {
        // Domain: [10, 20, 30]. Interval [1, 5] => entirely below.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(1)], vec![Some(5)]);
        assert_eq!(result, vec![Some(false)]);
    }

    #[test]
    fn test_above_domain() {
        // Domain: [10, 20, 30]. Interval [31, 40] => entirely above.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(31)], vec![Some(40)]);
        assert_eq!(result, vec![Some(false)]);
    }

    #[test]
    fn test_exact_lower_bound() {
        // Domain: [10, 20, 30]. Interval [10, 10] => exact hit at lower bound.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(10)], vec![Some(10)]);
        assert_eq!(result, vec![Some(true)]);
    }

    #[test]
    fn test_exact_upper_bound() {
        // Domain: [10, 20, 30]. Interval [30, 30] => exact hit at upper bound.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(30)], vec![Some(30)]);
        assert_eq!(result, vec![Some(true)]);
    }

    #[test]
    fn test_null_min() {
        // Null min, max=25: domain has values <= 25 (10, 20) => unknown.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![None], vec![Some(25)]);
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn test_null_min_provably_above() {
        // Null min, max=5: all domain values > 5 => can prune.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![None], vec![Some(5)]);
        assert_eq!(result, vec![Some(false)]);
    }

    #[test]
    fn test_null_max() {
        // Min=15, null max: domain has values >= 15 (20, 30) => unknown.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(15)], vec![None]);
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn test_null_max_provably_below() {
        // Min=35, null max: all domain values < 35 => can prune.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(35)], vec![None]);
        assert_eq!(result, vec![Some(false)]);
    }

    #[test]
    fn test_both_null() {
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![None], vec![None]);
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn test_inverted_range() {
        // min > max is invalid stats => unknown.
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(30)], vec![Some(10)]);
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn test_multiple_containers() {
        let expr = make_expr(vec![10, 20, 30], DataType::Int64);
        let result = eval_with_int64(
            &expr,
            vec![Some(10), Some(11), Some(25), Some(31)],
            vec![Some(10), Some(19), Some(29), Some(40)],
        );
        assert_eq!(
            result,
            vec![Some(true), Some(false), Some(false), Some(false)]
        );
    }

    #[test]
    fn test_dedup_and_sort() {
        // Unsorted, duplicated input should still work.
        let expr = make_expr(vec![30, 10, 20, 10, 30], DataType::Int64);
        let result = eval_with_int64(&expr, vec![Some(15)], vec![Some(25)]);
        assert_eq!(result, vec![Some(true)]);
    }

    #[test]
    fn test_single_value_domain() {
        let expr = make_expr(vec![42], DataType::Int64);
        let result = eval_with_int64(
            &expr,
            vec![Some(40), Some(42), Some(43)],
            vec![Some(41), Some(42), Some(50)],
        );
        assert_eq!(result, vec![Some(false), Some(true), Some(false)]);
    }

    #[test]
    fn test_dictionary_encoded_stats() {
        // Test with dictionary-encoded min/max arrays (common in Parquet).
        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "min",
                DataType::Dictionary(
                    Box::new(DataType::Int32),
                    Box::new(DataType::Int64),
                ),
                true,
            ),
            Field::new(
                "max",
                DataType::Dictionary(
                    Box::new(DataType::Int32),
                    Box::new(DataType::Int64),
                ),
                true,
            ),
        ]));
        let values_arr = Int64Array::from(vec![15, 25]);
        let keys = arrow::array::Int32Array::from(vec![0, 1]);
        let min_dict: ArrayRef = Arc::new(
            DictionaryArray::<DictKeyType>::try_new(
                keys.clone(),
                Arc::new(values_arr.clone()),
            )
            .unwrap(),
        );
        let max_values = Int64Array::from(vec![18, 30]);
        let max_dict: ArrayRef = Arc::new(
            DictionaryArray::<DictKeyType>::try_new(keys, Arc::new(max_values)).unwrap(),
        );
        let batch =
            RecordBatch::try_new(schema.clone(), vec![min_dict, max_dict]).unwrap();

        let min_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        let expr = IntegerInListPruningExpr::new(
            min_col,
            max_col,
            vec![10, 20, 30],
            DataType::Int64,
        );
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(2).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        // [15, 18] => gap between 10 and 20 => false
        // [25, 30] => contains 30 => true
        assert!(!bool_arr.value(0));
        assert!(bool_arr.value(1));
    }

    #[test]
    fn test_date32_type() {
        // Test with Date32 statistics arrays.
        let schema = Arc::new(Schema::new(vec![
            Field::new("min", DataType::Date32, true),
            Field::new("max", DataType::Date32, true),
        ]));
        let min_arr: ArrayRef = Arc::new(Date32Array::from(vec![Some(100), Some(200)]));
        let max_arr: ArrayRef = Arc::new(Date32Array::from(vec![Some(150), Some(250)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![min_arr, max_arr]).unwrap();

        let min_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        let expr = IntegerInListPruningExpr::new(
            min_col,
            max_col,
            vec![120, 300],
            DataType::Date32,
        );
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(2).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        // [100, 150] contains 120 => true
        // [200, 250] does not contain 120 or 300 => false
        assert!(bool_arr.value(0));
        assert!(!bool_arr.value(1));
    }

    #[test]
    fn test_timestamp_type() {
        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "min",
                DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                true,
            ),
            Field::new(
                "max",
                DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                true,
            ),
        ]));
        let min_arr: ArrayRef = Arc::new(TimestampMicrosecondArray::from(vec![
            Some(1000),
            Some(3000),
        ]));
        let max_arr: ArrayRef = Arc::new(TimestampMicrosecondArray::from(vec![
            Some(2000),
            Some(4000),
        ]));
        let batch = RecordBatch::try_new(schema.clone(), vec![min_arr, max_arr]).unwrap();

        let min_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        let expr = IntegerInListPruningExpr::new(
            min_col,
            max_col,
            vec![1500, 5000],
            DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
        );
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(2).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        // [1000, 2000] contains 1500 => true
        // [3000, 4000] does not contain 1500 or 5000 => false
        assert!(bool_arr.value(0));
        assert!(!bool_arr.value(1));
    }

    #[test]
    fn test_uint64_large_values() {
        // Test that large unsigned values are handled correctly (no overflow).
        let schema = Arc::new(Schema::new(vec![
            Field::new("min", DataType::UInt64, true),
            Field::new("max", DataType::UInt64, true),
        ]));
        let big = u64::MAX - 10;
        let min_arr: ArrayRef = Arc::new(UInt64Array::from(vec![Some(big)]));
        let max_arr: ArrayRef = Arc::new(UInt64Array::from(vec![Some(u64::MAX)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![min_arr, max_arr]).unwrap();

        let min_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        let expr = IntegerInListPruningExpr::new(
            min_col,
            max_col,
            vec![u64::MAX as i128 - 5],
            DataType::UInt64,
        );
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(1).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(bool_arr.value(0));
    }

    #[test]
    fn test_int8_type() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("min", DataType::Int8, true),
            Field::new("max", DataType::Int8, true),
        ]));
        let min_arr: ArrayRef = Arc::new(Int8Array::from(vec![Some(-100), Some(50)]));
        let max_arr: ArrayRef = Arc::new(Int8Array::from(vec![Some(-50), Some(100)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![min_arr, max_arr]).unwrap();

        let min_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("min", &schema).unwrap());
        let max_col: PhysicalExprRef =
            Arc::new(Column::new_with_schema("max", &schema).unwrap());
        let expr = IntegerInListPruningExpr::new(
            min_col,
            max_col,
            vec![-75, 60],
            DataType::Int8,
        );
        let result = expr.evaluate(&batch).unwrap();
        let arr = result.into_array(2).unwrap();
        let bool_arr = arr.as_any().downcast_ref::<BooleanArray>().unwrap();
        // [-100, -50] contains -75 => true
        // [50, 100] contains 60 => true
        assert!(bool_arr.value(0));
        assert!(bool_arr.value(1));
    }

    /// Verify compact pruning produces the same results as expanded OR would
    /// for a variety of intervals.
    #[test]
    fn test_equivalence_with_expanded_or() {
        let domain: Vec<i128> = vec![5, 15, 25, 35, 45, 55, 65, 75, 85, 95];
        let expr = make_expr(domain.clone(), DataType::Int64);

        // Test intervals: hits, gaps, boundaries, beyond range
        let test_intervals = vec![
            (0, 4, false),    // below all
            (5, 5, true),     // exact first
            (6, 14, false),   // gap
            (14, 16, true),   // straddles 15
            (95, 95, true),   // exact last
            (96, 100, false), // above all
            (0, 100, true),   // covers all
            (50, 60, true),   // contains 55
            (56, 64, false),  // gap between 55 and 65
        ];

        for (min, max, expected) in test_intervals {
            let result = eval_with_int64(&expr, vec![Some(min)], vec![Some(max)]);
            assert_eq!(
                result,
                vec![Some(expected)],
                "interval [{min}, {max}] expected {expected}"
            );
        }
    }
}
