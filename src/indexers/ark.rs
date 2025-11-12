// RGB ops library for working with smart contracts on Bitcoin & Lightning
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2024 by
//     RGB Protocol Contributors
//
// Copyright (C) 2024 LNP/BP Standards Association. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(feature = "ark")]
use std::collections::HashMap;
#[cfg(feature = "ark")]
use std::num::NonZeroU32;
#[cfg(feature = "ark")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "ark")]
use bp::{Tx, Txid};
#[cfg(feature = "ark")]
use rgbcore::validation::{ResolveWitness, WitnessResolverError, WitnessStatus};
#[cfg(feature = "ark")]
use rgbcore::vm::{WitnessOrd, WitnessPos};
#[cfg(feature = "ark")]
use rgbcore::ChainNet;

#[cfg(feature = "ark")]
use crate::containers::Consignment;

/// In-memory witness status tracker for MVP
#[cfg(feature = "ark")]
#[derive(Clone, Debug)]
struct WitnessTracker {
    /// Map of witness txid to (tx, status)
    witnesses: Arc<Mutex<HashMap<Txid, (Tx, WitnessOrd)>>>,
}

#[cfg(feature = "ark")]
impl WitnessTracker {
    fn new() -> Self {
        Self {
            witnesses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get(&self, txid: &Txid) -> Option<(Tx, WitnessOrd)> {
        self.witnesses.lock().unwrap().get(txid).cloned()
    }

    fn insert(&self, txid: Txid, tx: Tx, ord: WitnessOrd) {
        self.witnesses.lock().unwrap().insert(txid, (tx, ord));
    }

    fn update_ord(&self, txid: &Txid, ord: WitnessOrd) {
        if let Some((tx, _)) = self.witnesses.lock().unwrap().get(txid).cloned() {
            self.witnesses.lock().unwrap().insert(*txid, (tx, ord));
        }
    }
}

/// Ark resolver for RGB witness resolution.
/// 
/// For MVP, this uses an in-memory tracker seeded from consignments.
/// In a full implementation, this would query ark-client for transaction status.
#[cfg(feature = "ark")]
pub struct ArkResolver {
    /// In-memory witness tracker (MVP)
    tracker: WitnessTracker,
    /// Network to validate against
    network: ChainNet,
}

#[cfg(feature = "ark")]
impl ArkResolver {
    /// Create a new Ark resolver for the given network.
    pub fn new(network: ChainNet) -> Self {
        Self {
            tracker: WitnessTracker::new(),
            network,
        }
    }

    /// Add transactions from a consignment to the tracker.
    /// These will be marked as Tentative initially.
    pub fn add_consignment_txes<const TYPE: bool>(&mut self, consignment: &Consignment<TYPE>) {
        for bundle in &consignment.bundles {
            if let Some(tx) = bundle.pub_witness.tx() {
                let txid = tx.txid();
                self.tracker.insert(txid, tx.clone(), WitnessOrd::Tentative);
            }
        }
    }

    /// Track a witness transaction (seeds it as Tentative if not already tracked).
    pub fn track_witness(&mut self, txid: Txid, tx: Tx) {
        if self.tracker.get(&txid).is_none() {
            self.tracker.insert(txid, tx, WitnessOrd::Tentative);
        }
    }

    /// Poll once to update witness status (MVP: no-op, would query ark-client in full impl).
    /// For MVP, this is a placeholder that could be extended to query ark-client.
    pub fn poll_once(&mut self) -> Result<(), String> {
        // MVP: No-op. In full implementation, would query ark-client for transaction status
        // and update tracker with Confirmed status when transactions are mined.
        Ok(())
    }

    /// Update a witness to confirmed status (for testing/manual updates).
    pub fn mark_confirmed(&mut self, txid: &Txid, height: NonZeroU32, block_time: i64) -> Result<(), WitnessResolverError> {
        if let Some((tx, _)) = self.tracker.get(txid) {
            let pos = WitnessPos::bitcoin(height, block_time)
                .ok_or(WitnessResolverError::InvalidResolverData)?;
            self.tracker.update_ord(txid, WitnessOrd::Mined(pos));
            Ok(())
        } else {
            Err(WitnessResolverError::ResolverIssue(
                Some(*txid),
                s!("witness not tracked"),
            ))
        }
    }
}

#[cfg(feature = "ark")]
impl ResolveWitness for ArkResolver {
    fn check_chain_net(&self, chain_net: ChainNet) -> Result<(), WitnessResolverError> {
        if self.network != chain_net {
            return Err(WitnessResolverError::WrongChainNet);
        }
        Ok(())
    }

    fn resolve_witness(&self, witness_id: Txid) -> Result<WitnessStatus, WitnessResolverError> {
        match self.tracker.get(&witness_id) {
            Some((tx, ord)) => Ok(WitnessStatus::Resolved(tx, ord)),
            None => Ok(WitnessStatus::Unresolved),
        }
    }
}
