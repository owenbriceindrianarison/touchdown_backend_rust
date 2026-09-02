use chrono::{DateTime, Utc};
use contracts::auth::{AddressDto, AddressKind};
use uuid::Uuid;

/// Address entity.
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub kind: AddressKind,
    pub first_name: String,
    pub last_name: String,
    pub company: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub postal_code: String,
    pub city: String,
    pub state: Option<String>,
    pub country: String,
    pub phone: Option<String>,
    pub is_default_shipping: bool,
    pub is_default_billing: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Address {
    pub fn to_dto(&self) -> AddressDto {
        AddressDto {
            id: self.id,
            user_id: self.user_id,
            label: self.label.clone(),
            kind: self.kind,
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            company: self.company.clone(),
            line1: self.line1.clone(),
            line2: self.line2.clone(),
            postal_code: self.postal_code.clone(),
            city: self.city.clone(),
            state: self.state.clone(),
            country: self.country.clone(),
            phone: self.phone.clone(),
            is_default_shipping: self.is_default_shipping,
            is_default_billing: self.is_default_billing,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
