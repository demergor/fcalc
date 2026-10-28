# fcalc
## _Usage_: 
- **Normal Mode**: 
    - Select an operator to fold with and enter all the operands 
        (e.g. "- 6 5 5 1" would result in "6 - 5 - 5 - 1" to be calculated)
    - Currently defined operators: '+', '-', '/', '*', '\\', '^'
    - There are no parentheses, but you can define sub-expressions by positioning your
        cursor above the operand you want to base your expression off of and press 'e'
        to "enter" that operand
        Starting from there, you can define a contained sub-expression whose result
        will be positioned at the operand you "entered"
    - 'q', 'Q': quits the program, displaying the current global result
    - ENTER: evaluates the current (sub-) expression
    - '=': Evaluates the global expression
    - 'r': reverse the order of the operands (rfold; scoped to the current expression)
    - 'w': jumps forward (rhs) an operand (round-robin)
    - 'b': jumps backward (lhs) an operand (round-robin)
    - '_': invert the sign of the current operand
    - 'C': clears the global expression
    - 'c': clears the current (sub-) expression
    - 'v': enters Variable Selection mode
    - 'V': enters Variable Declaration mode
    - 'f': enters Function Selection mode (_not yet implemented_)
    - 'F': enters Function Declaration mode
- **Variable Selection Mode**: 
    - Select a predefined variable to enter into the current (sub-) expression 
    - ENTER: pastes the value of the currently selected variable
    - 'Q': quits the program, displaying the current global result
    - 'q': quits Variable Selection Mode, returning to Normal Mode
    - ARROW_UP, ARROW_DOWN: navigates through matching variables, shifting the current selection 
- **Variable Declaration Mode**:
    - Define a new variable (saved persistently into config at program exit)
    - ENTER: handle the variable declaration
    - 'D': enters deletion prefix to Variable-Declaration-command, upon pressing ENTER,
        the matching variable will be deleted
    - 'V': enters insertion prefix to Variable-Declaration-command, upon pressing ENTER,
        the variable's value will be inserted or updated if present

_The function modes are mostly still under construction_
_If you're stuck, try pressing 'h' (or 'H' in Function Declaration Mode), it might help_

Work In Progress: 
- add metafunctions to functions, like argc(...) to be able to make more complex function definitions possible
- add predefined functions like sum(x...), product(x...), log(x, y) (maybe add an operator for that as well), etc.
- implement smart cursor-locking when a render-cycle changes the underlying text buffer

Planned features:

Maybe features: 
- implement thousands delimiters

Bugs: 
- Typing in a subexpression in Function Declaration Mode crashes due to out-of-bounds 
    slice access
