/*
 * services/mfa/service.rs
 *
 * DEEPWELL - Wikijump API provider and database manager
 * Copyright (C) 2019-2022 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use super::prelude::*;
use crate::models::user::Model as UserModel;
use crate::services::{PasswordService, UserService};
use subtle::ConstantTimeEq;

/// The amount of time to give for each TOTP.
///
/// We use 30 seconds because this is standard with helpers
/// such as Google Authenticator and Authy.
///
/// It balances between giving the user enough time to enter a code,
/// but short enough to make bruteforcing values impractical.
const TIME_STEP: u64 = 30;

/// The allowed leniency value to account for clock skew.
///
/// This represents the seconds that a TOTP is offset by in
/// determining whether the authentication was accepted.
///
/// See https://github.com/TimDumol/rust-otp/blob/master/src/lib.rs#L56
const TIME_SKEW: i64 = 1;

#[derive(Debug)]
pub struct MfaService;

impl MfaService {
    pub async fn setup(ctx: &ServiceContext<'_>) -> Result<()> {
        let totp_secret = generate_totp_secret();
        let recovery_codes = RecoveryCodes::generate();

        todo!()
    }

    /// Verifies if the TOTP passed for this user is valid.
    ///
    /// # Returns
    /// Nothing on success, yields an `InvalidAuthentication` error on failure.
    pub async fn verify(
        ctx: &ServiceContext<'_>,
        user: &UserModel,
        entered_totp: u32,
    ) -> Result<()> {
        tide::log::info!("Verifying TOTP code for user ID {}", user.user_id);

        let secret = match &user.multi_factor_secret {
            Some(secret) => secret,
            None => {
                tide::log::warn!("User has no MFA secret, cannot verify TOTP");
                return Err(Error::InvalidAuthentication);
            }
        };

        let actual_totp = otp::make_totp(&secret, TIME_STEP, TIME_SKEW)?;

        // Constant-time comparison
        if actual_totp.ct_eq(&entered_totp).into() {
            Ok(())
        } else {
            Err(Error::InvalidAuthentication)
        }
    }

    /// Verifies if the recovery code for this user is valid.
    ///
    /// If it is, then the code is removed from the user's list
    /// of valid codes before returning success.
    ///
    /// # Returns
    /// Nothing on success, yields an `InvalidAuthentication` error on failure.
    pub async fn verify_recovery(
        ctx: &ServiceContext<'_>,
        user: &UserModel,
        recovery_code: &str,
    ) -> Result<()> {
        tide::log::info!("Verifying recovery code for user ID {}", user.user_id);

        let recovery_code_hashes = match &user.multi_factor_recovery_codes {
            Some(codes) => codes,
            None => {
                tide::log::warn!("User has no MFA recovery codes, but wants to verify recovery");
                return Err(Error::InvalidAuthentication);
            }
        };

        // Constant-time, check all the recovery codes even when we know we have a match.
        let mut result = Err(Error::InvalidAuthentication);
        for recovery_code_hash in recovery_code_hashes {
            if PasswordService::verify_sleep(recovery_code, &recovery_code_hash, false)
                .await
                .is_ok()
            {
                result = Ok(());
            }
        }

        // We sleep ourselves, once at the end.
        //
        // Otherwise we have variable-time recovery code checks based on whether
        // the recovery code was correct or not.
        if result.is_err() {
            PasswordService::failure_sleep().await;
        }

        result
    }
}
