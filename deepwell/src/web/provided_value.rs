/*
 * web/provided_value.rs
 *
 * DEEPWELL - Wikijump API provider and database manager
 * Copyright (C) 2021 Wikijump Team
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

/// Denotes that a field is optional in a struct.
///
/// This is meant to be used when doing `UPDATE` operations,
/// since excluding the field entirely is different from setting
/// it to null (`None`).
///
/// The `Unset` variant can only be constructed if the field is absent.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(untagged)]
pub enum ProvidedValue<T> {
    Set(T),

    #[serde(skip)]
    Unset,
}

impl<T> Default for ProvidedValue<T> {
    #[inline]
    fn default() -> Self {
        ProvidedValue::Unset
    }
}