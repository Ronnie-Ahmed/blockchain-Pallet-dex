use crate::pallet::{Config,Error};
use crate::{AssetBalanceOf,AssetIdOf};
use codec::{Decode,Encode,MaxEncodedLen};
use frame_support::dispatch::{DispatchResult, TypeInfo};
use frame_support::RuntimeDebug;
use std::marker::PhantomData;
use sp_runtime::traits::{CheckedAdd, CheckedSub};

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct LiquidityPool<T: Config> {
    pub assets: (AssetIdOf<T>, AssetIdOf<T>),
    pub reserves: (AssetBalanceOf<T>, AssetBalanceOf<T>),
    pub total_liquidity: AssetBalanceOf<T>,
    pub liquidity_token: AssetIdOf<T>,
    _marker: PhantomData<T>,
}

impl<T: Config> LiquidityPool<T> {
    // Function to mint liquidity tokens and update reserves
   pub fn mint(&mut self,amounts_in:(AssetBalanceOf<T>,AssetBalanceOf<T>),liquidity_minted:AssetBalanceOf<T>)->DispatchResult{
    self.reserves.0=self.reserves.0
                                .checked_add(&amounts_in.0)
                                .ok_or(Error::<T>::ReserveOverflow)?;
    self.reserves.1=self.reserves.1.checked_add(&amounts_in.0).ok_or(Error::<T>::ReserveOverflow)?;
    self.total_liquidity=self.total_liquidity.checked_add(&liquidity_minted).ok_or(Error::<T>::LiquidityOverflow)?;
    Ok(())
   }


   pub fn burn(&mut self,amounts_out:(AssetBalanceOf<T>,AssetBalanceOf<T>),liquidity_burned:AssetBalanceOf<T>)->DispatchResult{

    self.reserves.0=self.reserves.0.checked_sub(&amounts_out.0).ok_or(Error::<T>::InsufficientReserves)?;
    self.reserves.1=self.reserves.1.checked_sub(&amounts_out.1).ok_or(Error::<T>::InsufficientReserves)?;

    self.total_liquidity=self.total_liquidity.checked_sub(&liquidity_burned).ok_or(Error::<T>::InsufficientLiquidity)?;

    Ok(())
   }

   

}