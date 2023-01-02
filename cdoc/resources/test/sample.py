#| DOC
#| TASK {id=a, title=My Task} <<
def somefunc(a):
    #| CODE <<
    # ...
    # more code
    #| Markup here
    #| More markup
    #| >> SOLUTION <<

    # Comments
    #| Markup
    print(a)
    return 2*a

    #| >> END_CODE
#| >> END_TASK


#| TEST {task=a} <<
assert somefunc(5) == 10
assert somefunc(10) == 20
#| >> END_TEST
