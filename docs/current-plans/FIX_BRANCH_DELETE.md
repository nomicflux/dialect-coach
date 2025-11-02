# Fix branch deletion in UI

## Current behavior

Message stream:
User: A
Agent: B
User: C
Agent: D

User clicks after B for a new branch.
(User: A, Agent B; C+D in branch in sidebar)
User: X
Agent: Y

I click on the button to delete C+D.

C+D are deleted, as well as X+Y. Now conversation is just:
User: A
Agent: B

This is terrible.

## Desired behavior

Same message stream. We arrive at:
(User: A, Agent B; C+D in branch in sidebar)
User: X
Agent: Y

User clicks on delete button for C+D.
C+D are deleted. ONLY that subtree is deleted.

Current view of 
A
B
X
Y

is unaffected.

## More complex scenario, desired

Starting from:
(User: A, Agent B; C+D in branch in sidebar)
User: X
Agent: Y

User branches after X, writes a new message W. Now we have:
User: A
Agent: B   (C+D in branch in sidebar after this point)
User: X (Y in branch in sidebar after this point)
User: W

If the user clicks on the delete button for branch Y, we get:
User: A
Agent: B   (C+D _still_ in branch in sidebar after this point)
Uler: X (No more Y)
User: W

If the user clicks on the delete button for branch C+D, we get:
User: A
Agent: B  (deleted C+D)
User: X (Y _still_ in branch in sidebar after this point)
User: W

So, again, delete _affects the subtree deleted, deletes the whole subtree from that point, and only that subtree_
