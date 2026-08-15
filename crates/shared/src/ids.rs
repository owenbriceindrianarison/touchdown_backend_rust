use uuid::Uuid;

#[inline]
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}
