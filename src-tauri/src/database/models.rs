// SPDX-License-Identifier: AGPL-3.0-only
//! Struct data yang dipakai Commands dan service.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dataset {
    pub id:          i64,
    pub name:        String,
    pub description: Option<String>,
    pub domain:      Option<String>,
    pub grain_desc:  Option<String>,
    pub period_type: String,
    pub owner:       Option<String>,
    pub source:      Option<String>,
    pub status:      String,
    pub created_at:  String,
    pub updated_at:  String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuatDatasetInput {
    pub name:        String,
    pub description: Option<String>,
    pub domain:      Option<String>,
    pub grain_desc:  Option<String>,
    pub period_type: String,
    pub owner:       Option<String>,
    pub source:      Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UbahDatasetInput {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub domain:      Option<String>,
    pub grain_desc:  Option<String>,
    pub period_type: Option<String>,
    pub owner:       Option<String>,
    pub source:      Option<String>,
    pub reason:      Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Variable {
    pub id:          i64,
    pub dataset_id:  i64,
    pub name:        String,
    pub code:        String,
    pub concept:     Option<String>,
    pub definition:  Option<String>,
    pub data_type:   String,
    pub unit:        Option<String>,
    pub is_required: bool,
    pub sort_order:  i64,
    pub archived_at: Option<String>,
    pub created_at:  String,
    pub updated_at:  String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuatVariableInput {
    pub dataset_id:  i64,
    pub name:        String,
    pub code:        String,
    pub concept:     Option<String>,
    pub definition:  Option<String>,
    pub data_type:   String,
    pub unit:        Option<String>,
    pub is_required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UbahVariableInput {
    pub name:        Option<String>,
    pub concept:     Option<String>,
    pub definition:  Option<String>,
    pub unit:        Option<String>,
    pub is_required: Option<bool>,
    pub reason:      Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefUnit {
    pub id:        i64,
    pub code:      Option<String>,
    pub name:      String,
    pub level:     String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntriAuditLog {
    pub id:          i64,
    pub entity_type: String,
    pub entity_id:   i64,
    pub action:      String,
    pub field:       Option<String>,
    pub old_value:   Option<String>,
    pub new_value:   Option<String>,
    pub reason:      Option<String>,
    pub changed_by:  String,
    pub changed_at:  String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub id:   i64,
    pub name: String,
    pub description: Option<String>,
}
