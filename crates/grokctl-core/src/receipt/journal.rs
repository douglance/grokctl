//! `SQLite` mutation receipt journal.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use super::{MutationReceipt, ReceiptState};

/// Receipt journal failure.
#[derive(Debug, Error)]
pub enum JournalError {
    /// `SQLite` failed.
    #[error("receipt journal failed: {0}")]
    Sql(#[from] rusqlite::Error),
    /// Existing key was used with a different command or input.
    #[error("idempotency key conflicts with an existing receipt")]
    Conflict,
}

/// SQLite-backed local mutation journal.
#[derive(Clone, Debug)]
pub struct ReceiptJournal {
    path: PathBuf,
}

impl ReceiptJournal {
    /// Open or create a journal at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError`] if `SQLite` cannot initialize the schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let journal = Self { path: path.as_ref().to_owned() };
        journal.initialize()?;
        Ok(journal)
    }

    /// Read one receipt by gateway and idempotency key.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError`] if `SQLite` cannot execute the query.
    pub fn get(
        &self,
        gateway_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<MutationReceipt>, JournalError> {
        let connection = Connection::open(&self.path)?;
        let mut query = connection.prepare(
            "SELECT command, input_hash, state, request_id, response_hash \
             FROM mutation_receipts WHERE gateway_id = ?1 AND idempotency_key = ?2",
        )?;
        let receipt = query
            .query_row(params![gateway_id, idempotency_key], |row| {
                Ok(MutationReceipt {
                    gateway_id: gateway_id.to_owned(),
                    command: row.get(0)?,
                    idempotency_key: idempotency_key.to_owned(),
                    input_hash: row.get(1)?,
                    state: parse_state(&row.get::<_, String>(2)?),
                    request_id: row.get(3)?,
                    response_hash: row.get(4)?,
                })
            })
            .optional()?;
        Ok(receipt)
    }

    /// Insert a newly admitted mutation receipt.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::Conflict`] when the key already witnesses different input.
    pub fn begin(&self, receipt: &MutationReceipt) -> Result<MutationReceipt, JournalError> {
        if let Some(existing) = self.get(&receipt.gateway_id, &receipt.idempotency_key)? {
            return replay_existing(existing, receipt);
        }
        let connection = Connection::open(&self.path)?;
        connection.execute(
            "INSERT INTO mutation_receipts \
             (gateway_id, command, idempotency_key, input_hash, state, request_id, response_hash) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                receipt.gateway_id,
                receipt.command,
                receipt.idempotency_key,
                receipt.input_hash,
                state_name(receipt.state),
                receipt.request_id,
                receipt.response_hash,
            ],
        )?;
        Ok(receipt.clone())
    }

    /// Mark a receipt as successfully applied.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError`] if `SQLite` cannot update the record.
    pub fn mark_succeeded(
        &self,
        gateway_id: &str,
        idempotency_key: &str,
        response_hash: &str,
    ) -> Result<(), JournalError> {
        self.update(gateway_id, idempotency_key, ReceiptState::Succeeded, Some(response_hash))
    }

    /// Mark a receipt ambiguous after a transport failure.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError`] if `SQLite` cannot update the record.
    pub fn mark_ambiguous(
        &self,
        gateway_id: &str,
        idempotency_key: &str,
    ) -> Result<(), JournalError> {
        self.update(gateway_id, idempotency_key, ReceiptState::Ambiguous, None)
    }

    fn update(
        &self,
        gateway_id: &str,
        idempotency_key: &str,
        state: ReceiptState,
        response_hash: Option<&str>,
    ) -> Result<(), JournalError> {
        let connection = Connection::open(&self.path)?;
        connection.execute(
            "UPDATE mutation_receipts SET state = ?3, request_id = ?4, response_hash = ?5 \
             WHERE gateway_id = ?1 AND idempotency_key = ?2",
            params![
                gateway_id,
                idempotency_key,
                state_name(state),
                Option::<&str>::None,
                response_hash
            ],
        )?;
        Ok(())
    }

    fn initialize(&self) -> Result<(), JournalError> {
        let connection = Connection::open(&self.path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS mutation_receipts (\
             gateway_id TEXT NOT NULL, command TEXT NOT NULL, idempotency_key TEXT NOT NULL, \
             input_hash TEXT NOT NULL, state TEXT NOT NULL, request_id TEXT, response_hash TEXT, \
             PRIMARY KEY (gateway_id, idempotency_key));",
        )?;
        Ok(())
    }
}

fn replay_existing(
    existing: MutationReceipt,
    requested: &MutationReceipt,
) -> Result<MutationReceipt, JournalError> {
    if existing.command != requested.command || existing.input_hash != requested.input_hash {
        Err(JournalError::Conflict)
    } else {
        Ok(existing)
    }
}

const fn state_name(state: ReceiptState) -> &'static str {
    match state {
        ReceiptState::Started => "started",
        ReceiptState::Succeeded => "succeeded",
        ReceiptState::Ambiguous => "ambiguous",
        ReceiptState::ResolvedApplied => "resolved_applied",
        ReceiptState::ResolvedNotApplied => "resolved_not_applied",
    }
}

fn parse_state(value: &str) -> ReceiptState {
    match value {
        "succeeded" => ReceiptState::Succeeded,
        "ambiguous" => ReceiptState::Ambiguous,
        "resolved_applied" => ReceiptState::ResolvedApplied,
        "resolved_not_applied" => ReceiptState::ResolvedNotApplied,
        _ => ReceiptState::Started,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn replays_same_witness_and_rejects_conflicts() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Some(directory) = directory.ok() else { return };
        let journal = ReceiptJournal::open(directory.path().join("receipts.db"));
        assert!(journal.is_ok());
        let Some(journal) = journal.ok() else { return };
        let receipt = fixture("hash-a");
        assert!(journal.begin(&receipt).is_ok());
        assert_eq!(journal.begin(&receipt).ok(), Some(receipt.clone()));
        let conflict = journal.begin(&fixture("hash-b"));
        assert!(matches!(conflict, Err(JournalError::Conflict)));
    }

    fn fixture(input_hash: &str) -> MutationReceipt {
        MutationReceipt {
            gateway_id: "gateway".to_owned(),
            command: "createAgent".to_owned(),
            idempotency_key: "key".to_owned(),
            input_hash: input_hash.to_owned(),
            state: ReceiptState::Started,
            request_id: None,
            response_hash: None,
        }
    }
}
