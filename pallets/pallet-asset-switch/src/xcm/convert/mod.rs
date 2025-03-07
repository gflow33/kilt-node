// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2024 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org

use sp_runtime::{traits::TryConvert, AccountId32};
use xcm::v4::Junction::{self, AccountId32 as AccountId32Junction};

const LOG_TARGET: &str = "xcm::pallet-asset-switch::AccountId32ToAccountId32JunctionConverter";

/// Type implementing `TryConvert<AccountId32, Junction>` and returns a
/// `Junction` from an `AccountId32`.
pub struct AccountId32ToAccountId32JunctionConverter;

impl TryConvert<AccountId32, Junction> for AccountId32ToAccountId32JunctionConverter {
	fn try_convert(account: AccountId32) -> Result<Junction, AccountId32> {
		log::info!(target: LOG_TARGET, "try_convert {:?}", account);
		Ok(AccountId32Junction {
			network: None,
			id: account.into(),
		})
	}
}
