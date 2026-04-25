/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

pub mod layers;
pub mod variants;

// Re-export commonly used items
pub use variants::{add_ecu_shared_data, add_functional_groups, add_protocols, add_variants};
