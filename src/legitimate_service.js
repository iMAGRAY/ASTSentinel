// Real service implementation
async function getUserData(userId) {
  const response = await fetch(`/api/users/${userId}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch user: ${response.status}`);
  }
  return response.json();
}

function calculateTotal(items) {
  return items.reduce((sum, item) => {
    const price = item.price * item.quantity;
    const tax = price * 0.1;
    return sum + price + tax;
  }, 0);
}

module.exports = { getUserData, calculateTotal };