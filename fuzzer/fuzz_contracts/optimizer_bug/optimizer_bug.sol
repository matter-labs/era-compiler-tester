contract test {
  function function_to_fuzz(uint8 a, bool b) external pure returns (uint8) {
      uint8 c = b ? 1 : 0;
      uint8 d = a + c; // this arithmetic incorrectly throws an overflow panic
      return b ? c : d;
  }
}
// ----
