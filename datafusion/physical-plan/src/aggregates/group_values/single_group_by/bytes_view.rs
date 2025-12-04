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

use crate::aggregates::group_values::GroupValues;
use arrow::array::{Array, ArrayRef, RecordBatch};
use datafusion_common::internal_err;
use datafusion_expr::EmitTo;
use datafusion_physical_expr::binary_map::OutputType;
use datafusion_physical_expr_common::binary_view_map::ArrowBytesViewMap;
use std::mem::size_of;

/// A [`GroupValues`] storing single column of Utf8View/BinaryView values
///
/// This specialization is significantly faster than using the more general
/// purpose `Row`s format
pub struct GroupValuesBytesView {
    /// Map string/binary values to group index
    map: ArrowBytesViewMap<usize>,
    /// The total number of groups so far (used to assign group_index)
    num_groups: usize,
    /// Block size for blocked emission (None = flat mode)
    block_size: Option<usize>,
    /// State for blocked emission: stores remaining data to emit
    emit_state: Option<ArrayRef>,
}

impl GroupValuesBytesView {
    pub fn new(output_type: OutputType) -> Self {
        Self {
            map: ArrowBytesViewMap::new(output_type),
            num_groups: 0,
            block_size: None,
            emit_state: None,
        }
    }
}

impl GroupValues for GroupValuesBytesView {
    fn intern(
        &mut self,
        cols: &[ArrayRef],
        groups: &mut Vec<usize>,
    ) -> datafusion_common::Result<()> {
        if self.emit_state.is_some() {
            return internal_err!("can not update groups during blocks emitting");
        }
        
        assert_eq!(cols.len(), 1);

        // look up / add entries in the table
        let arr = &cols[0];

        groups.clear();
        self.map.insert_if_new(
            arr,
            // called for each new group
            |_value| {
                // assign new group index on each insert
                let group_idx = self.num_groups;
                self.num_groups += 1;
                group_idx
            },
            // called for each group
            |group_idx| {
                groups.push(group_idx);
            },
        );

        // ensure we assigned a group to for each row
        assert_eq!(groups.len(), arr.len());
        Ok(())
    }

    fn size(&self) -> usize {
        self.map.size() + size_of::<Self>()
    }

    fn is_empty(&self) -> bool {
        self.num_groups == 0
    }

    fn len(&self) -> usize {
        self.num_groups
    }

    fn emit(&mut self, emit_to: EmitTo) -> datafusion_common::Result<Vec<ArrayRef>> {
        let group_values = match emit_to {
            EmitTo::All => {
                assert!(self.block_size.is_none(), "only support EmitTo::All in flat mode");
                let map_contents = self.map.take().into_state();
                self.num_groups -= map_contents.len();
                map_contents
            }
            EmitTo::First(n) if n == self.len() => {
                assert!(self.block_size.is_none(), "only support EmitTo::First in flat mode");
                let map_contents = self.map.take().into_state();
                self.num_groups -= map_contents.len();
                map_contents
            }
            EmitTo::First(n) => {
                assert!(self.block_size.is_none(), "only support EmitTo::First in flat mode");
                let map_contents = self.map.take().into_state();
                // if we only wanted to take the first n, insert the rest back
                // into the map we could potentially avoid this reallocation, at
                // the expense of much more complex code.
                // see https://github.com/apache/datafusion/issues/9195
                let emit_group_values = map_contents.slice(0, n);
                let remaining_group_values =
                    map_contents.slice(n, map_contents.len() - n);

                self.num_groups = 0;
                let mut group_indexes = vec![];
                self.intern(&[remaining_group_values], &mut group_indexes)?;

                // Verify that the group indexes were assigned in the correct order
                assert_eq!(0, group_indexes[0]);

                emit_group_values
            }
            EmitTo::NextBlock => {
                let block_size = self.block_size.expect("only support EmitTo::NextBlock in blocked mode");
                
                // First call: initialize emit state
                if self.emit_state.is_none() {
                    let map_contents = self.map.take().into_state();
                    self.emit_state = Some(map_contents);
                }
                
                // Get the remaining data
                let remaining = self.emit_state.as_ref().unwrap();
                let remaining_len = remaining.len();
                
                if remaining_len == 0 {
                    // All blocks emitted
                    self.emit_state = None;
                    return internal_err!("try to evaluate empty group values");
                }
                
                // Emit one block
                let emit_len = block_size.min(remaining_len);
                let emit_group_values = remaining.slice(0, emit_len);
                
                // Update state
                if emit_len < remaining_len {
                    let new_remaining = remaining.slice(emit_len, remaining_len - emit_len);
                    self.emit_state = Some(new_remaining);
                } else {
                    // Last block emitted
                    self.emit_state = None;
                }
                
                self.num_groups -= emit_len;
                emit_group_values
            }
        };

        Ok(vec![group_values])
    }

    fn clear_shrink(&mut self, _batch: &RecordBatch) {
        // in theory we could potentially avoid this reallocation and clear the
        // contents of the maps, but for now we just reset the map from the beginning
        self.map.take();
        self.emit_state = None;
        self.num_groups = 0;
    }
    
    fn supports_blocked_groups(&self) -> bool {
        true
    }
    
    fn alter_block_size(&mut self, block_size: Option<usize>) -> datafusion_common::Result<()> {
        self.map.take();
        self.emit_state = None;
        self.num_groups = 0;
        self.block_size = block_size;
        Ok(())
    }
}
