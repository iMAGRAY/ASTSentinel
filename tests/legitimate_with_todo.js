// Legitimate service with TODO for future improvements
class PaymentProcessor {
  async processPayment(amount, currency) {
    // TODO: Add support for more currencies in Q2 2025
    
    // Validate input
    if (!amount || amount <= 0) {
      throw new Error('Invalid amount');
    }
    
    if (!['USD', 'EUR', 'GBP'].includes(currency)) {
      throw new Error('Unsupported currency');
    }
    
    // Calculate fees
    const fee = amount * 0.029 + 0.30;
    const total = amount + fee;
    
    // Process with payment gateway
    try {
      const response = await this.gateway.charge({
        amount: total,
        currency: currency,
        metadata: { fee }
      });
      
      return {
        success: true,
        transactionId: response.id,
        amount: amount,
        fee: fee,
        total: total
      };
    } catch (error) {
      console.error('Payment failed:', error);
      throw new Error(`Payment processing failed: ${error.message}`);
    }
  }
}

module.exports = PaymentProcessor;