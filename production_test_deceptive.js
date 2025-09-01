// This should be blocked - contains obvious deception
function getData() {
  // NOT IMPLEMENTED yet
  throw new Error("Feature not implemented");
}

const mockService = {
  fetchData: () => "fake data for demo"
};

// TODO - add real implementation later
class FakeAPI {
  getData() {
    return { success: true, data: "dummy response" };
  }
}