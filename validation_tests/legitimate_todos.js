// BASELINE TEST: Legitimate TODOs that SHOULD be allowed
class PaymentService {
  constructor(gateway) {
    this.gateway = gateway;
  }

  // TODO: (JIRA-1234) Add retry mechanism after infrastructure upgrade
  async processPayment(amount) {
    if (!amount || amount <= 0) {
      throw new Error("Invalid amount");
    }
    return await this.gateway.charge(amount);
  }

  // TODO: #567 Optimize database queries for performance
  async getTransactions(userId) {
    return await this.db.query('SELECT * FROM transactions WHERE user_id = ?', [userId]);
  }

  // TODO: (@john.doe) Review security implications before release
  validateCardDetails(card) {
    return card && card.number && card.expiry && card.cvv;
  }

  // TODO: (2024-06-15) Migrate to new payment API after vendor update
  formatResponse(data) {
    return {
      success: true,
      data: data,
      timestamp: new Date().toISOString()
    };
  }

  // TODO: Add caching layer when Redis is deployed
  getCachedUser(id) {
    // Will implement caching after infrastructure is ready
    return this.userRepo.findById(id);
  }

  // TODO: Optimize performance after migrating to microservices
  calculateFees(amount) {
    const baseFee = 0.03;
    return Math.round(amount * baseFee * 100) / 100;
  }
}