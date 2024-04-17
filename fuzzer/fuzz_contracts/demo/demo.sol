contract test {

    function should_always_return_0(uint8 input) public pure returns (uint8) {
        if (input == 10) {
            return 1;
        }
        return 0;
    }

}
// ----
