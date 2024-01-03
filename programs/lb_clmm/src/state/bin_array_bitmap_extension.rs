use crate::constants::{BIN_ARRAY_BITMAP_SIZE, EXTENSION_BINARRAY_BITMAP_SIZE};
use crate::errors::LBError;
use crate::math::safe_math::SafeMath;
use crate::math::utils_math::one;
use anchor_lang::prelude::*;
use ruint::aliases::U512;
use std::ops::BitXor;

#[account(zero_copy)]
#[derive(Debug, InitSpace)]
pub struct BinArrayBitmapExtension {
    pub lb_pair: Pubkey,
    /// Packed initialized bin array state for start_bin_index is positive
    pub positive_bin_array_bitmap: [[u64; 8]; EXTENSION_BINARRAY_BITMAP_SIZE],
    /// Packed initialized bin array state for start_bin_index is negative
    pub negative_bin_array_bitmap: [[u64; 8]; EXTENSION_BINARRAY_BITMAP_SIZE],
}

impl Default for BinArrayBitmapExtension {
    #[inline]
    fn default() -> BinArrayBitmapExtension {
        BinArrayBitmapExtension {
            lb_pair: Pubkey::default(),
            positive_bin_array_bitmap: [[0; 8]; EXTENSION_BINARRAY_BITMAP_SIZE],
            negative_bin_array_bitmap: [[0; 8]; EXTENSION_BINARRAY_BITMAP_SIZE],
        }
    }
}

impl BinArrayBitmapExtension {
    pub fn initialize(&mut self, lb_pair: Pubkey) {
        self.lb_pair = lb_pair;
        self.positive_bin_array_bitmap = [[0; 8]; EXTENSION_BINARRAY_BITMAP_SIZE];
        self.negative_bin_array_bitmap = [[0; 8]; EXTENSION_BINARRAY_BITMAP_SIZE];
    }

    fn get_bitmap_offset(bin_array_index: i32) -> Result<usize> {
        // bin_array_index starts from 512 in positive side and -513 in negative side
        let offset = if bin_array_index > 0 {
            bin_array_index / BIN_ARRAY_BITMAP_SIZE - 1
        } else {
            -(bin_array_index + 1) / BIN_ARRAY_BITMAP_SIZE - 1
        };
        Ok(offset as usize)
    }

    /// According to the given bin array index, calculate its corresponding binarray and then find the bitmap it belongs to.
    fn get_bitmap(&self, bin_array_index: i32) -> Result<(usize, [u64; 8])> {
        let offset = Self::get_bitmap_offset(bin_array_index)?;
        if bin_array_index < 0 {
            Ok((offset, self.negative_bin_array_bitmap[offset]))
        } else {
            Ok((offset, self.positive_bin_array_bitmap[offset]))
        }
    }

    fn bin_array_offset_in_bitmap(bin_array_index: i32) -> Result<usize> {
        if bin_array_index > 0 {
            Ok(bin_array_index.safe_rem(BIN_ARRAY_BITMAP_SIZE)? as usize)
        } else {
            Ok((-(bin_array_index + 1)).safe_rem(BIN_ARRAY_BITMAP_SIZE)? as usize)
        }
    }

    fn to_bin_array_index(
        offset: usize,
        bin_array_offset: usize,
        is_positive: bool,
    ) -> Result<i32> {
        let offset = offset as i32;
        let bin_array_offset = bin_array_offset as i32;
        if is_positive {
            Ok((offset + 1) * BIN_ARRAY_BITMAP_SIZE + bin_array_offset)
        } else {
            Ok(-((offset + 1) * BIN_ARRAY_BITMAP_SIZE + bin_array_offset) - 1)
        }
    }

    /// Flip the value of bin in the bitmap.
    pub fn flip_bin_array_bit(&mut self, bin_array_index: i32) -> Result<()> {
        // TODO do we need validate bin_array_index again?
        let (offset, bin_array_bitmap) = self.get_bitmap(bin_array_index).unwrap();
        let bin_array_offset_in_bitmap = Self::bin_array_offset_in_bitmap(bin_array_index).unwrap();
        let bin_array_bitmap = U512::from_limbs(bin_array_bitmap);

        let mask = one::<512, 8>() << bin_array_offset_in_bitmap;
        if bin_array_index < 0 {
            self.negative_bin_array_bitmap[offset as usize] =
                bin_array_bitmap.bitxor(mask).into_limbs();
        } else {
            self.positive_bin_array_bitmap[offset as usize] =
                bin_array_bitmap.bitxor(mask).into_limbs();
        }
        Ok(())
    }

    pub fn bit(&self, bin_array_index: i32) -> Result<bool> {
        let (_, bin_array_bitmap) = self.get_bitmap(bin_array_index)?;
        let bin_array_offset_in_bitmap = Self::bin_array_offset_in_bitmap(bin_array_index)?;
        let bin_array_bitmap = U512::from_limbs(bin_array_bitmap);
        return Ok(bin_array_bitmap.bit(bin_array_offset_in_bitmap as usize));
    }
    pub fn bitmap_range() -> (i32, i32) {
        return (
            -BIN_ARRAY_BITMAP_SIZE * (EXTENSION_BINARRAY_BITMAP_SIZE as i32 + 1),
            BIN_ARRAY_BITMAP_SIZE * (EXTENSION_BINARRAY_BITMAP_SIZE as i32 + 1) - 1,
        );
    }

    pub fn iter_bitmap(&self, start_index: i32, end_index: i32) -> Result<Option<i32>> {
        let offset: usize = Self::get_bitmap_offset(start_index)?;
        let bin_array_offset = Self::bin_array_offset_in_bitmap(start_index)?;
        if start_index < 0 {
            // iter in negative_bin_array_bitmap
            if start_index <= end_index {
                for i in (0..=offset).rev() {
                    let mut bin_array_bitmap = U512::from_limbs(self.negative_bin_array_bitmap[i]);

                    if i == offset {
                        bin_array_bitmap = bin_array_bitmap
                            << BIN_ARRAY_BITMAP_SIZE as usize - bin_array_offset - 1;
                        if bin_array_bitmap.eq(&U512::ZERO) {
                            continue;
                        }

                        let bin_array_offset_in_bitmap =
                            bin_array_offset - bin_array_bitmap.leading_zeros();

                        return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                            i,
                            bin_array_offset_in_bitmap,
                            false,
                        )?));
                    }
                    if bin_array_bitmap.eq(&U512::ZERO) {
                        continue;
                    }
                    let bin_array_offset_in_bitmap =
                        BIN_ARRAY_BITMAP_SIZE as usize - bin_array_bitmap.leading_zeros() - 1;
                    return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                        i,
                        bin_array_offset_in_bitmap,
                        false,
                    )?));
                }
            } else {
                for i in offset..EXTENSION_BINARRAY_BITMAP_SIZE {
                    let mut bin_array_bitmap = U512::from_limbs(self.negative_bin_array_bitmap[i]);
                    if i == offset {
                        bin_array_bitmap = bin_array_bitmap >> bin_array_offset;
                        if bin_array_bitmap.eq(&U512::ZERO) {
                            continue;
                        }

                        let bin_array_offset_in_bitmap =
                            bin_array_offset + bin_array_bitmap.trailing_zeros();

                        return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                            i,
                            bin_array_offset_in_bitmap,
                            false,
                        )?));
                    }

                    if bin_array_bitmap.eq(&U512::ZERO) {
                        continue;
                    }
                    let bin_array_offset_in_bitmap = bin_array_bitmap.trailing_zeros();

                    return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                        i,
                        bin_array_offset_in_bitmap,
                        false,
                    )?));
                }
            }
        } else {
            // iter in possitive_bin_array_bitmap
            if start_index <= end_index {
                for i in offset..EXTENSION_BINARRAY_BITMAP_SIZE {
                    let mut bin_array_bitmap = U512::from_limbs(self.positive_bin_array_bitmap[i]);
                    if i == offset {
                        bin_array_bitmap = bin_array_bitmap >> bin_array_offset;
                        if bin_array_bitmap.eq(&U512::ZERO) {
                            continue;
                        }

                        let bin_array_offset_in_bitmap =
                            bin_array_offset + bin_array_bitmap.trailing_zeros();
                        return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                            i,
                            bin_array_offset_in_bitmap,
                            true,
                        )?));
                    }

                    if bin_array_bitmap.eq(&U512::ZERO) {
                        continue;
                    }

                    let bin_array_offset_in_bitmap = bin_array_bitmap.trailing_zeros();
                    return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                        i,
                        bin_array_offset_in_bitmap,
                        true,
                    )?));
                }
            } else {
                for i in (0..=offset).rev() {
                    let mut bin_array_bitmap = U512::from_limbs(self.positive_bin_array_bitmap[i]);

                    if i == offset {
                        bin_array_bitmap = bin_array_bitmap
                            << BIN_ARRAY_BITMAP_SIZE as usize - bin_array_offset - 1;

                        if bin_array_bitmap.eq(&U512::ZERO) {
                            continue;
                        }
                        let bin_array_offset_in_bitmap =
                            bin_array_offset - bin_array_bitmap.leading_zeros();
                        return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                            i,
                            bin_array_offset_in_bitmap,
                            true,
                        )?));
                    }

                    if bin_array_bitmap.eq(&U512::ZERO) {
                        continue;
                    }
                    let bin_array_offset_in_bitmap =
                        BIN_ARRAY_BITMAP_SIZE as usize - bin_array_bitmap.leading_zeros() - 1;
                    return Ok(Some(BinArrayBitmapExtension::to_bin_array_index(
                        i,
                        bin_array_offset_in_bitmap,
                        true,
                    )?));
                }
            }
        }
        Ok(None)
    }

    pub fn next_bin_array_index_with_liquidity(
        &self,
        swap_for_y: bool,
        start_index: i32,
    ) -> Result<(i32, bool)> {
        let (min_bitmap_id, max_bit_map_id) = BinArrayBitmapExtension::bitmap_range();
        if start_index > 0 {
            if swap_for_y {
                match self.iter_bitmap(start_index, BIN_ARRAY_BITMAP_SIZE)? {
                    Some(value) => return Ok((value, true)),
                    None => return Ok((BIN_ARRAY_BITMAP_SIZE - 1, false)),
                }
            } else {
                match self.iter_bitmap(start_index, max_bit_map_id)? {
                    Some(value) => return Ok((value, true)),
                    None => return Err(LBError::CannotFindNonZeroLiquidityBinArrayId.into()),
                }
            }
        } else {
            if swap_for_y {
                match self.iter_bitmap(start_index, min_bitmap_id)? {
                    Some(value) => return Ok((value, true)),
                    None => return Err(LBError::CannotFindNonZeroLiquidityBinArrayId.into()),
                }
            } else {
                match self.iter_bitmap(start_index, -BIN_ARRAY_BITMAP_SIZE - 1)? {
                    Some(value) => return Ok((value, true)),
                    None => return Ok((-BIN_ARRAY_BITMAP_SIZE, false)),
                }
            }
        }
    }
}

#[cfg(test)]
pub mod bin_array_bitmap_extension_test {
    use crate::{
        constants::{MAX_BIN_ID, MAX_BIN_PER_ARRAY},
        state::lb_pair::LbPair,
    };
    use core::panic;
    use proptest::proptest;

    use super::*;

    #[test]
    fn test_flip_bin_array_bit_extension() {
        let mut extension = BinArrayBitmapExtension::default();
        let bin_array_index = BIN_ARRAY_BITMAP_SIZE;
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), true);
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), false);

        let bin_array_index = -BIN_ARRAY_BITMAP_SIZE - 1;
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), true);
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), false);

        // max range
        let bin_array_index = MAX_BIN_ID / (MAX_BIN_PER_ARRAY as i32) + 1;
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), true);

        let bin_array_index = -MAX_BIN_ID / (MAX_BIN_PER_ARRAY as i32) - 1;
        extension.flip_bin_array_bit(bin_array_index).unwrap();
        assert_eq!(extension.bit(bin_array_index).unwrap(), true);
    }

    #[test]
    fn test_flip_all_bin_array_bit_extension() {
        let mut extension = BinArrayBitmapExtension::default();
        let max_bin_array_index = MAX_BIN_ID / (MAX_BIN_PER_ARRAY as i32) + 1;
        let min_bin_array_index = -MAX_BIN_ID / (MAX_BIN_PER_ARRAY as i32) - 1;

        for i in BIN_ARRAY_BITMAP_SIZE..max_bin_array_index {
            extension.flip_bin_array_bit(i).unwrap();
            assert_eq!(extension.bit(i).unwrap(), true);
        }
        for i in min_bin_array_index..(-BIN_ARRAY_BITMAP_SIZE) {
            extension.flip_bin_array_bit(i).unwrap();
            assert_eq!(extension.bit(i).unwrap(), true);
        }

        for i in BIN_ARRAY_BITMAP_SIZE..max_bin_array_index {
            extension.flip_bin_array_bit(i).unwrap();
            assert_eq!(extension.bit(i).unwrap(), false);
        }
        for i in min_bin_array_index..(-BIN_ARRAY_BITMAP_SIZE) {
            extension.flip_bin_array_bit(i).unwrap();
            assert_eq!(extension.bit(i).unwrap(), false);
        }
    }

    #[test]
    fn test_next_id_to_initialized_bin_array_from_internal_to_extension_swap_for_x() {
        let mut extension = BinArrayBitmapExtension::default();

        let (_, max_bin_array_index) = BinArrayBitmapExtension::bitmap_range();
        let start_index = BIN_ARRAY_BITMAP_SIZE;

        let index = 2000;
        // deposit liquidity at index 2000
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);
        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(false, start_index)
            .unwrap();
        assert_eq!(index, bin_array_id);

        assert_eq!(ok, true);
        // swap for x
        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(false, start_index)
            .unwrap();
        assert_eq!(index, bin_array_id);
        assert_eq!(ok, true);
        // withdraw liquidity at index 2000
        extension.flip_bin_array_bit(index).unwrap();

        let index = max_bin_array_index;
        // deposit liquidity at index max_bin_array_index
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);

        // swap for x
        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(false, start_index)
            .unwrap();
        assert_eq!(index, bin_array_id);
        assert_eq!(ok, true);

        // if we dont find non zero liquidity, then we have to return error
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), false);

        match extension.next_bin_array_index_with_liquidity(false, start_index) {
            Ok(_value) => panic!("should panic"),
            Err(_err) => {}
        };
    }

    #[test]
    fn test_next_id_to_initialized_bin_array_from_internal_to_extension_swap_for_y() {
        let mut extension = BinArrayBitmapExtension::default();
        let (min_bin_array_index, _) = BinArrayBitmapExtension::bitmap_range();
        let start_index = -BIN_ARRAY_BITMAP_SIZE - 1;
        let index = -2000;
        // deposit liquidity at index -2000
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);

        // swap for x
        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(true, start_index)
            .unwrap();
        assert_eq!(index, bin_array_id);
        assert_eq!(ok, true);

        // withdraw liquidity at index -2000
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), false);

        let index = min_bin_array_index;
        // deposit liquidity at index min_bin_array_index
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);

        // swap for x
        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(true, start_index)
            .unwrap();

        assert_eq!(index, bin_array_id);
        assert_eq!(ok, true);

        // if we dont find non zero liquidity, then we have to return error
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), false);
        match extension.next_bin_array_index_with_liquidity(true, start_index) {
            Ok(_value) => panic!("should panic"),
            Err(_err) => {}
        };
    }

    #[test]
    fn test_next_id_to_initialized_bin_array_from_extension_to_internal_swap_for_y() {
        let mut extension = BinArrayBitmapExtension::default();
        let index: i32 = 2000;
        let start_index = index - 1;
        // deposit liquidity at index 2000
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);

        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(true, start_index)
            .unwrap();
        assert_eq!(ok, false);
        assert_eq!(bin_array_id, BIN_ARRAY_BITMAP_SIZE - 1);
    }

    #[test]
    fn test_next_id_to_initialized_bin_array_from_extension_to_internal_swap_for_x() {
        let mut extension = BinArrayBitmapExtension::default();
        let index: i32 = -2000;
        let start_index = index + 1;
        // deposit liquidity at index 2000
        extension.flip_bin_array_bit(index).unwrap();
        assert_eq!(extension.bit(index).unwrap(), true);

        let (bin_array_id, ok) = extension
            .next_bin_array_index_with_liquidity(false, start_index)
            .unwrap();
        assert_eq!(ok, false);
        assert_eq!(bin_array_id, -BIN_ARRAY_BITMAP_SIZE);
    }

    #[test]
    fn test_bin_array_offset() {
        let (min_bin_id, max_bin_id) = LbPair::bitmap_range();

        let next_max_bin_id = max_bin_id + 1;
        let bitmap_offset = BinArrayBitmapExtension::get_bitmap_offset(next_max_bin_id).unwrap();
        assert_eq!(bitmap_offset, 0);
        let bin_array_offset =
            BinArrayBitmapExtension::bin_array_offset_in_bitmap(next_max_bin_id).unwrap();
        assert_eq!(bin_array_offset, 0);

        let end_nex_max_bin_id = next_max_bin_id + 511;
        let bitmap_offset = BinArrayBitmapExtension::get_bitmap_offset(end_nex_max_bin_id).unwrap();
        assert_eq!(bitmap_offset, 0);
        let bin_array_offset =
            BinArrayBitmapExtension::bin_array_offset_in_bitmap(end_nex_max_bin_id).unwrap();
        assert_eq!(bin_array_offset, 511);

        let next_min_bin_id = min_bin_id - 1;
        let bitmap_offset = BinArrayBitmapExtension::get_bitmap_offset(next_min_bin_id).unwrap();
        assert_eq!(bitmap_offset, 0);
        let bin_array_offset =
            BinArrayBitmapExtension::bin_array_offset_in_bitmap(next_min_bin_id).unwrap();
        assert_eq!(bin_array_offset, 0);

        let end_nex_min_bin_id = next_min_bin_id - 511;
        let bitmap_offset = BinArrayBitmapExtension::get_bitmap_offset(end_nex_min_bin_id).unwrap();
        assert_eq!(bitmap_offset, 0);
        let bin_array_offset =
            BinArrayBitmapExtension::bin_array_offset_in_bitmap(end_nex_min_bin_id).unwrap();
        assert_eq!(bin_array_offset, 511);
    }

    #[test]
    fn test_iter_map() {
        let mut extension = BinArrayBitmapExtension::default();
        extension.flip_bin_array_bit(-1111).unwrap();
        extension.flip_bin_array_bit(-2222).unwrap();
        extension.flip_bin_array_bit(-2225).unwrap();

        extension.flip_bin_array_bit(1111).unwrap();
        extension.flip_bin_array_bit(2222).unwrap();
        extension.flip_bin_array_bit(2225).unwrap();

        assert_eq!(extension.iter_bitmap(-513, -5555).unwrap().unwrap(), -1111);
        assert_eq!(extension.iter_bitmap(-5555, -513).unwrap().unwrap(), -2225);

        assert_eq!(extension.iter_bitmap(513, 5555).unwrap().unwrap(), 1111);
        assert_eq!(extension.iter_bitmap(5555, 513).unwrap().unwrap(), 2225);
    }

    #[test]
    fn test_iter_map_ajacent_items_negative_index() {
        let mut extension = BinArrayBitmapExtension::default();
        extension.flip_bin_array_bit(-1111).unwrap();
        extension.flip_bin_array_bit(-1113).unwrap();
        extension.flip_bin_array_bit(-1115).unwrap();
        assert_eq!(extension.iter_bitmap(-1115, -1111).unwrap().unwrap(), -1115);
        assert_eq!(extension.iter_bitmap(-1114, -1111).unwrap().unwrap(), -1113);
        assert_eq!(extension.iter_bitmap(-1111, -1115).unwrap().unwrap(), -1111);
        assert_eq!(extension.iter_bitmap(-1112, -1115).unwrap().unwrap(), -1113);
    }

    #[test]
    fn test_iter_map_ajacent_items_possitive_index() {
        let mut extension = BinArrayBitmapExtension::default();
        extension.flip_bin_array_bit(1111).unwrap();
        extension.flip_bin_array_bit(1113).unwrap();
        extension.flip_bin_array_bit(1115).unwrap();

        assert_eq!(extension.iter_bitmap(1111, 1115).unwrap().unwrap(), 1111);
        assert_eq!(extension.iter_bitmap(1112, 1115).unwrap().unwrap(), 1113);
        assert_eq!(extension.iter_bitmap(1115, 1111).unwrap().unwrap(), 1115);
        assert_eq!(extension.iter_bitmap(1114, 1111).unwrap().unwrap(), 1113);
    }

    proptest! {
        #[test]
        fn test_next_possitive_bin_array_index_with_liquidity(
            swap_for_y in 0..=1,
            start_index in 512..6655,
            flip_id in 512..6655

        ) {
            let mut extension = BinArrayBitmapExtension::default();
            extension.flip_bin_array_bit(flip_id).unwrap();
            assert_eq!(extension.bit(flip_id).unwrap(), true);

            let swap_for_y = if swap_for_y == 0 {
                false
            }else{
                true
            };
            let result = extension.next_bin_array_index_with_liquidity(swap_for_y, start_index);

            if swap_for_y {
                let (bin_id, ok) = result.unwrap();
                if start_index >= flip_id {
                    assert_eq!(bin_id, flip_id);
                    assert_eq!(ok, true);
                }else{
                    assert_eq!(bin_id, 511);
                    assert_eq!(ok, false);
                }
            }else{
                if start_index > flip_id {
                    assert_eq!(result.is_err(), true);
                }else{
                    let (bin_id, ok) = result.unwrap();
                    assert_eq!(bin_id, flip_id);
                    assert_eq!(ok, true);

                }
            }
        }

        #[test]
        fn test_next_negative_bin_array_index_with_liquidity(
            swap_for_y in 0..=1,
            start_index in -6656..-513,
            flip_id in -6656..-513

        ) {
            let mut extension = BinArrayBitmapExtension::default();
            extension.flip_bin_array_bit(flip_id).unwrap();
            assert_eq!(extension.bit(flip_id).unwrap(), true);

            let swap_for_y = if swap_for_y == 0 {
                false
            }else{
                true
            };
            let result = extension.next_bin_array_index_with_liquidity(swap_for_y, start_index);

            if swap_for_y {

                if start_index < flip_id {
                    assert_eq!(result.is_err(), true);
                }else{
                    let (bin_id, ok) = result.unwrap();
                    assert_eq!(bin_id, flip_id);
                    assert_eq!(ok, true);
                }
            }else{
                let (bin_id, ok) = result.unwrap();
                if start_index <= flip_id {
                    assert_eq!(bin_id, flip_id);
                    assert_eq!(ok, true);
                }else{
                    assert_eq!(bin_id, -512);
                    assert_eq!(ok, false);

                }
            }
        }
    }
}