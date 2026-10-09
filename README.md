Work In Progress: 
- fix bugs:
    Repeatedly entering 0 results in "0 0 0 0" instead of "0000000" in `FunctionHandler`
    1'var_name converts to 'var_name automatically and leaves the cur_col out of sync
    in `FunctionHandler`

- add functions 
- add metafunctions to those functions, like argc(...) to be able to make more complex function definitions possible
- add predefined functions like sum(x...), product(x...), log(x, y) (maybe add an operator for that as well), etc.
- implement smart cursor-locking when a render-cycle changes the underlying text buffer

Planned features:

Maybe features: 
- implement thousands delimiters
