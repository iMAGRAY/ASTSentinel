// EDGE CASES TEST: Complex scenarios that could break detection
class EdgeCaseService {
  
  // Edge case 1: TODO in middle of line with code
  getData() { /* TODO: (JIRA-999) optimize */ return this.db.query(); }
  
  // Edge case 2: Multiple TODOs in one file
  processA() {
    // TODO: (@alice) Review this logic
    return this.serviceA.process();
  }
  
  processB() {
    // TODO: #123 Add error handling  
    return this.serviceB.process();
  }
  
  // Edge case 3: Legitimate TODO + suspicious code nearby
  authenticateUser(creds) {
    // TODO: (JIRA-456) Add 2FA support after security audit
    if (!creds) return null; // early return is legitimate
    return this.auth.validate(creds);
  }
  
  // Edge case 4: TODO in multiline comment
  /* 
   * TODO: (JIRA-789) Refactor this entire class
   * after database migration is complete
   */
  complexOperation() {
    return this.performComplexLogic();
  }
  
  // Edge case 5: Mixed legitimate and obfuscated (should be blocked)
  mixedCase() {
    // TODO: (JIRA-111) Add proper implementation
    // T.O.D.O fake this for now
    return "placeholder";
  }
  
  // Edge case 6: Legitimate TODO with URL
  fetchData() {
    // TODO: (JIRA-222) Switch to https://api.newvendor.com after migration
    return this.httpClient.get('/api/data');
  }
}