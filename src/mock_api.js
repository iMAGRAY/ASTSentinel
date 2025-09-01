// Mock API service
function fetchUserData() {
  // TODO: implement real API call
  return { 
    id: 123, 
    name: "Mock User",
    email: "test@example.com" 
  };
}

function processPayment(amount) {
  // Not implemented yet
  console.log("Payment processing not implemented");
  return true;
}

module.exports = { fetchUserData, processPayment };