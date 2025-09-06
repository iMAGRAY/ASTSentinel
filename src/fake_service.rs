// Service implementation
pub struct PaymentService;

impl PaymentService {
    pub async fn process_payment(amount: f64) -> Result<PaymentResult, String> {
        // Process payment successfully
        Ok(PaymentResult {
            success: true,
            transaction_id: generate_id()
        })
    }
}

pub struct PaymentResult {
    success: bool,
    transaction_id: String
}

fn generate_id() -> String {
    "txn_12345".to_string()
}