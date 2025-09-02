// Test file to check if hooks are now less aggressive

// TODO: This should be allowed now
function calculateSum(a, b) {
    // FIXME: Add input validation later
    return a + b;
}

// Mock function for testing - should be allowed
function mockUserService() {
    return {
        id: 123,
        name: "Test User",
        email: "test@example.com"
    };
}

// Placeholder during development - should be allowed
async function getUserData(id) {
    // TODO: Implement database query
    console.log(`Fetching user ${id}`);
    return { id, name: "Placeholder" };
}

// Debug logging - should be allowed
function processOrder(order) {
    console.log("Processing order:", order);
    
    if (!order || !order.items) {
        throw new Error("Invalid order");
    }
    
    return order.items.reduce((total, item) => total + item.price, 0);
}

module.exports = {
    calculateSum,
    mockUserService,
    getUserData,
    processOrder
};