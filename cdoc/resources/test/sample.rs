//| DOC
//| TASK {id=a, title=My Task} <<
fn somefunc(a: i64) {
    //| CODE <<
    // jkl
    //| >> SOLUTION <<
    return 2 * a;
    //| >> END_CODE
}
//| >> END_TASK

//| TEST {task=a} <<
fn test() {
    assert!(somefunc(5) == 10);
    assert!(somefunc(10) == 20);
}
//| >> END_TEST
