// BASELINE TEST: Deceptive TODOs that MUST be blocked
class FakeService {
  // TODO - add implementation later (vague, no tracking)
  process() {
    return true; // placeholder
  }

  // T.O.D.O implement this (obfuscated)
  validate() {
    return true;
  }

  // T O D O fake response (spaced obfuscation)
  getData() {
    return "mock data";
  }

  // TODO: finish me (disguised)
  save() {
    // NOT IMPLEMENTED
    throw new Error("not implemented");
  }

  // FIXME: complete this later (vague disguised)
  update() {
    return { success: true }; // hardcoded
  }
}