// This should be allowed
class DataService {
  constructor(database) {
    this.db = database;
  }
  
  async fetchUser(id) {
    if (!id) throw new Error("ID required");
    return await this.db.query("SELECT * FROM users WHERE id = ?", [id]);
  }
}

module.exports = DataService;