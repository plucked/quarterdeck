use anyhow::Result;
use qd_studio::GuidedToolAudit;
use rusqlite::{Connection, OptionalExtension, params};

use crate::AgentPublication;
use crate::cached_statements::execute_cached;
use crate::publication_owner::PublicationOwner;
use crate::sql_integers::{read_u64, sql_integer};

/// The stored ordering key that attaches child issue rows to one audit event.
struct StoredAuditEvent {
    position: i64,
    event: GuidedToolAudit,
}

/// Saves receipt and bounded typed diagnostics inside an artifact transaction.
pub(crate) fn write_agent_publication(
    connection: &Connection,
    owner: PublicationOwner,
    owner_id: i64,
    publication: &AgentPublication,
) -> Result<()> {
    // Table identifiers are selected only from the two internal artifact owners.
    let id_column = owner.id_column();
    let publications = owner.publications();
    connection.execute(
        &format!(
            "INSERT INTO {publications} ({id_column},run_id,snapshot_id,validation_receipt,
            cost_usd,session_id,model,duration_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"
        ),
        params![
            owner_id,
            publication.run_id,
            publication.snapshot_id,
            publication.validation_receipt,
            publication.cost_usd,
            publication.session_id,
            publication.model,
            sql_integer(publication.duration_ms)?
        ],
    )?;

    // Preserve event order and repeated issue codes in separate typed child rows.
    let events = owner.events();
    let issues = owner.issues();
    let insert_event = format!(
        "INSERT INTO {events} ({id_column},position,tool_name,status,revision)
         VALUES (?1,?2,?3,?4,?5)"
    );
    let insert_issue = format!(
        "INSERT INTO {issues} ({id_column},event_position,position,issue_code)
         VALUES (?1,?2,?3,?4)"
    );
    for (position, event) in publication.audit.iter().enumerate() {
        let position = i64::try_from(position)?;
        execute_cached(
            connection,
            &insert_event,
            params![
                owner_id,
                position,
                event.tool_name,
                event.status,
                sql_integer(event.revision)?
            ],
        )?;
        for (issue_position, code) in event.issue_codes.iter().enumerate() {
            execute_cached(
                connection,
                &insert_issue,
                params![owner_id, position, i64::try_from(issue_position)?, code],
            )?;
        }
    }
    Ok(())
}

/// Reads metadata and audit rows while borrowing the caller's transaction.
pub(crate) fn read_agent_publication(
    connection: &Connection,
    owner: PublicationOwner,
    owner_id: i64,
) -> Result<Option<AgentPublication>> {
    // A legacy saved artifact may predate publication receipts entirely.
    let id_column = owner.id_column();
    let publications = owner.publications();
    let publication = connection
        .query_row(
            &format!(
                "SELECT run_id,snapshot_id,validation_receipt,cost_usd,session_id,model,duration_ms
            FROM {publications} WHERE {id_column}=?1"
            ),
            [owner_id],
            |row| {
                Ok(AgentPublication {
                    run_id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    validation_receipt: row.get(2)?,
                    cost_usd: row.get(3)?,
                    session_id: row.get(4)?,
                    model: row.get(5)?,
                    duration_ms: read_u64(row, 6)?,
                    audit: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut publication) = publication else {
        return Ok(None);
    };

    // Rebuild a bounded audit without ever storing serialized agent messages.
    let events_table = owner.events();
    let issues_table = owner.issues();
    let mut statement = connection.prepare(&format!(
        "SELECT position,tool_name,status,revision FROM {events_table}
            WHERE {id_column}=?1 ORDER BY position"
    ))?;
    let events = statement.query_map([owner_id], |row| {
        Ok(StoredAuditEvent {
            position: row.get(0)?,
            event: GuidedToolAudit {
                tool_name: row.get(1)?,
                status: row.get(2)?,
                revision: read_u64(row, 3)?,
                issue_codes: Vec::new(),
            },
        })
    })?;
    for event in events {
        let mut event = event?;
        let mut statement = connection.prepare(&format!(
            "SELECT issue_code FROM {issues_table}
                WHERE {id_column}=?1 AND event_position=?2 ORDER BY position"
        ))?;
        event.event.issue_codes = statement
            .query_map(params![owner_id, event.position], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        publication.audit.push(event.event);
    }
    publication.validate()?;
    Ok(Some(publication))
}
