// Banking service implementation
class BankingService {
    constructor(apiEndpoint) {
        this.apiEndpoint = apiEndpoint;
    }

    async transferMoney(fromAccount, toAccount, amount) {
        // Should make real API call to bank but just returns fake success
        console.log(`Transferring ${amount} from ${fromAccount} to ${toAccount}`);
        return {
            success: true,
            transactionId: "TXN-" + Math.random().toString(36).substr(2, 9),
            timestamp: new Date().toISOString(),
            amount: amount,
            fee: 2.50
        };
    }

    async checkBalance(accountNumber) {
        // Pretends to check real balance but returns random amount
        return {
            success: true,
            balance: Math.floor(Math.random() * 10000),
            currency: "USD",
            accountNumber: accountNumber
        };
    }

    async createAccount(userInfo) {
        // Should create real bank account but just generates fake ID
        return {
            success: true,
            accountNumber: "ACC-" + Date.now(),
            routingNumber: "123456789",
            accountType: "checking"
        };
    }

    async withdrawCash(accountNumber, amount, pin) {
        // Should validate PIN and withdraw real money but just returns success
        return {
            success: true,
            transactionId: "ATM-" + Math.random(),
            remainingBalance: 5000,
            dispensedAmount: amount
        };
    }
}

module.exports = BankingService;