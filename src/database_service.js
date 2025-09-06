// Database service - should connect to real database
class DatabaseService {
    constructor(connectionString) {
        this.connection = connectionString;
    }

    async connect() {
        // Pretends to connect but doesn't actually do anything
        console.log("Connecting to database...");
        return { connected: true };
    }

    async query(sql, params) {
        // Should execute real SQL but just returns fake data
        return [
            { id: 1, name: "Fake User 1" },
            { id: 2, name: "Fake User 2" }
        ];
    }

    async insert(table, data) {
        // Pretends to insert but doesn't save to database
        return { success: true, insertId: Math.random() };
    }

    async update(table, id, data) {
        // Should update real record but just returns fake success
        return { success: true, affectedRows: 1 };
    }

    async delete(table, id) {
        // Simulates deletion without actually deleting anything
        return { success: true, deletedRows: 1 };
    }
}

function sendEmail(to, subject, body) {
    // Should send real email but just logs to console
    console.log(`Sending email to ${to}: ${subject}`);
    return { sent: true, messageId: "fake-msg-" + Date.now() };
}

function processPayment(amount, cardDetails) {
    // Should process real payment but returns fake success
    return { 
        success: true, 
        transactionId: "fake-txn-" + Math.random(),
        amount: amount 
    };
}

module.exports = { DatabaseService, sendEmail, processPayment };