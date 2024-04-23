
#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::pallet_prelude::*;
use frame_support::traits::fungible;
use frame_support::traits::fungibles;
use frame_support::PalletId;
use pallet::*;
use sp_runtime::traits::{
    AccountIdConversion, CheckedDiv, CheckedMul, IntegerSquareRoot, Saturating, Zero,
};



#[cfg(test)]
mod mock;


mod liquidity_pool;
#[cfg(test)]
mod tests;


pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::Balance;


#[frame_support::pallet]
pub mod pallet {
    
    use super::*;
    use crate::liquidity_pool::LiquidityPool;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::fungibles::Mutate;
    use frame_support::traits::tokens::{Fortitude, Precision, Preservation};
    use frame_system::pallet_prelude::*;

    
    
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    
    #[pallet::config]
    pub trait Config: frame_system::Config {
        
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        
        type NativeBalance: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<Self::AccountId>
            + fungible::freeze::Inspect<Self::AccountId>
            + fungible::freeze::Mutate<Self::AccountId>;

        
        type Fungibles: fungibles::Inspect<Self::AccountId, AssetId = u32>
            + fungibles::Mutate<Self::AccountId>
            + fungibles::Create<Self::AccountId>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    
    #[pallet::storage]
    pub type LiquidityPools<T: Config> =
        StorageMap<_, Blake2_128Concat, (AssetIdOf<T>, AssetIdOf<T>), LiquidityPool<T>>;

    
    #[pallet::storage]
    pub type LiquidityTokens<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetIdOf<T>, (AssetIdOf<T>, AssetIdOf<T>), ValueQuery>;

    
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        
        
        
        
        LiquidityPoolCreated(AccountIdOf<T>, (AssetIdOf<T>, AssetIdOf<T>)),

        
        
        
        
        
        LiquidityMinted(
            AccountIdOf<T>,
            (AssetIdOf<T>, AssetIdOf<T>),
            AssetBalanceOf<T>,
        ),

        
        
        
        
        
        LiquidityBurned(
            AccountIdOf<T>,
            (AssetIdOf<T>, AssetIdOf<T>),
            AssetBalanceOf<T>,
        ),
    }

    
    #[pallet::error]
    pub enum Error<T> {
        
        InsufficientLiquidity,

        
        InsufficientReserves,

        
        ReserveOverflow,

        
        LiquidityOverflow,

        
        InvalidAssetIn,

        
        InvalidAssetOut,

        
        InsufficientAmountOut,

        
        ArithmeticOverflow,

        
        DivisionByZero,

        
        LiquidityPoolAlreadyExists,

        
        LiquidityPoolNotFound,

        
        InsufficientLiquidityMinted,

        
        InsufficientAmountsOut,

        
        ZeroLiquidityBurned,
    }

    
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        
      #[pallet::call_index(0)]
      #[pallet::weight(Weight::default())]
      pub fn create_liquidity_pool(
        origin: OriginFor<T>,
        asset_a: AssetIdOf<T>,
        asset_b: AssetIdOf<T>,
        liquidity_token: AssetIdOf<T>,
    ) -> DispatchResult {
        let sender = ensure_signed(origin)?;
        let trading_pair=(asset_a,asset_b);
        ensure!(!LiquidityPools::<T>::contains_key(trading_pair),Error::<T>::LiquidityPoolAlreadyExists);

        let pool=LiquidityPool{
            assets:trading_pair,
            reserves:(Zero::zero(),Zero::zero()),
            total_liquidity:Zero::zero(),
            liquidity_token,
        };
        LiquidityPools::<T>::insert(trading_pair,pool);
        Self::deposit_event(Event::LiquidityPoolCreated(sender, trading_pair));

            Ok(())

        }



        #[pallet::call_index(1)]
        #[pallet::weight(Weight::default())]
        pub fn mint_liquidity(
            origin: OriginFor<T>,
            asset_a: AssetIdOf<T>,
            asset_b: AssetIdOf<T>,
            amount_a: AssetBalanceOf<T>,
            amount_b: AssetBalanceOf<T>,
            min_liquidity: AssetBalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let trading_pair = (asset_a, asset_b);

            
            let mut liquidity_pool =
                LiquidityPools::<T>::get(&trading_pair).ok_or(Error::<T>::LiquidityPoolNotFound)?;

            
            let liquidity_minted = Self::calculate_liquidity_minted(
                (amount_a, amount_b),
                (liquidity_pool.reserves.0, liquidity_pool.reserves.1),
                liquidity_pool.total_liquidity,
            )?;

            
            ensure!(
                liquidity_minted >= min_liquidity,
                Error::<T>::InsufficientLiquidityMinted
            );

            
            Self::transfer_asset_to_pool(&sender, trading_pair.0, amount_a)?;
            Self::transfer_asset_to_pool(&sender, trading_pair.1, amount_b)?;

            
            Self::mint_liquidity_tokens(&sender, liquidity_pool.liquidity_token, liquidity_minted)?;

            
            liquidity_pool.mint((amount_a, amount_b), liquidity_minted)?;

            
            LiquidityPools::<T>::insert(&trading_pair, liquidity_pool);

            
            Self::deposit_event(Event::LiquidityMinted(
                sender,
                trading_pair,
                liquidity_minted,
            ));

            Ok(())
        }

        
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::default())]
        pub fn burn_liquidity(
            origin: OriginFor<T>,
            asset_a: AssetIdOf<T>,
            asset_b: AssetIdOf<T>,
            liquidity_burned: AssetBalanceOf<T>,
            min_amount_a: AssetBalanceOf<T>,
            min_amount_b: AssetBalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let trading_pair = (asset_a, asset_b);

            let mut liquidity_pool =
                LiquidityPools::<T>::get(trading_pair).ok_or(Error::<T>::LiquidityPoolNotFound)?;

            
            
            let amounts_out = Self::calculate_amounts_out(
                liquidity_burned,
                (liquidity_pool.reserves.0, liquidity_pool.reserves.1),
                liquidity_pool.total_liquidity,
            )?;
            ensure!(
                amounts_out.0 >= min_amount_a && amounts_out.1 >= min_amount_b,
                Error::<T>::InsufficientAmountsOut
            );

            
            Self::burn_liquidity_tokens(&sender, liquidity_pool.liquidity_token, liquidity_burned)?;

            
            liquidity_pool.burn(liquidity_burned, amounts_out)?;
            LiquidityPools::<T>::insert(trading_pair, liquidity_pool);

            Self::deposit_event(Event::LiquidityBurned(
                sender,
                trading_pair,
                liquidity_burned,
            ));

            Ok(())
        }
    }

    
    impl<T: Config> Pallet<T> {
        fn calculate_liquidity_minted(
            amounts: (AssetBalanceOf<T>, AssetBalanceOf<T>),
            reserves: (AssetBalanceOf<T>, AssetBalanceOf<T>),
            total_liquidity: AssetBalanceOf<T>,
        ) -> Result<AssetBalanceOf<T>, DispatchError> {
            let (amount_a, amount_b) = amounts;
            let (reserve_a, reserve_b) = reserves;

            ensure!(
                !amount_a.is_zero() && !amount_b.is_zero(),
                Error::<T>::InsufficientLiquidityMinted
            );

            if total_liquidity.is_zero() {
                
                let liquidity_minted = Self::geometric_mean(amount_a, amount_b)?;
                Ok(liquidity_minted)
            } else {
                
                let liquidity_minted_a = amount_a
                    .checked_mul(&total_liquidity)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .checked_div(&reserve_a)
                    .ok_or(Error::<T>::DivisionByZero)?;

                let liquidity_minted_b = amount_b
                    .checked_mul(&total_liquidity)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .checked_div(&reserve_b)
                    .ok_or(Error::<T>::DivisionByZero)?;

                
                let liquidity_minted = sp_std::cmp::min(liquidity_minted_a, liquidity_minted_b);
                Ok(liquidity_minted)
            }
        }

        fn geometric_mean(
            amount_a: AssetBalanceOf<T>,
            amount_b: AssetBalanceOf<T>,
        ) -> Result<AssetBalanceOf<T>, DispatchError> {
            let sqrt_product = (amount_a
                .checked_mul(&amount_b)
                .ok_or(Error::<T>::ArithmeticOverflow)?)
            .integer_sqrt();
            Ok(sqrt_product)
        }

        fn transfer_asset_to_pool(
            sender: &AccountIdOf<T>,
            asset_id: AssetIdOf<T>,
            amount: AssetBalanceOf<T>,
        ) -> DispatchResult {
            let pool_account_id = T::PalletId::get().into_account_truncating();

            
            T::Fungibles::transfer(
                asset_id,
                sender,
                &pool_account_id,
                amount,
                Preservation::Expendable,
            )?;

            Ok(())
        }

        fn mint_liquidity_tokens(
            recipient: &AccountIdOf<T>,
            liquidity_token_id: AssetIdOf<T>,
            amount: AssetBalanceOf<T>,
        ) -> DispatchResult {
            
            T::Fungibles::mint_into(liquidity_token_id, recipient, amount)?;
            Ok(())
        }

        fn burn_liquidity_tokens(
            sender: &AccountIdOf<T>,
            liquidity_token_id: AssetIdOf<T>,
            amount: AssetBalanceOf<T>,
        ) -> DispatchResult {
            
            T::Fungibles::burn_from(
                liquidity_token_id,
                sender,
                amount,
                Precision::Exact,
                Fortitude::Polite,
            )?;
            Ok(())
        }

        fn calculate_amounts_out(
            liquidity_burned: AssetBalanceOf<T>,
            reserves: (AssetBalanceOf<T>, AssetBalanceOf<T>),
            total_liquidity: AssetBalanceOf<T>,
        ) -> Result<(AssetBalanceOf<T>, AssetBalanceOf<T>), DispatchError> {
            ensure!(!liquidity_burned.is_zero(), Error::<T>::ZeroLiquidityBurned);
            ensure!(
                !total_liquidity.is_zero(),
                Error::<T>::InsufficientLiquidity
            );

            let (reserve_a, reserve_b) = reserves;
            ensure!(
                !reserve_a.is_zero() && !reserve_b.is_zero(),
                Error::<T>::InsufficientLiquidity
            );

            let amount_a = liquidity_burned
                .checked_mul(&reserve_a)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(&total_liquidity)
                .ok_or(Error::<T>::DivisionByZero)?;

            let amount_b = liquidity_burned
                .checked_mul(&reserve_b)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(&total_liquidity)
                .ok_or(Error::<T>::DivisionByZero)?;

            Ok((amount_a, amount_b))
        }
    }
}
