// Our base LinkedList representation
// We can make value generic later
// When next is None we're at the end of the list
// In order to create recursive types we have to store fixed sized types on the stack
// We use a Box to create a fixed size pointer on the heap pointing to the heap
struct LinkedList {
    value: i32,
    next: Option<Box<LinkedList>>,
}

// This wrapper type lets us implement an unconsumed .iter() method on LinkedList
// Note: because next_node contains a reference, we have to have an explicit lifetime annotation
struct LinkedListIterRef<'a> {
    next_node: Option<&'a LinkedList>,
}

// This wrapper type lets us implement IntoIterator on our LinkedList
// This lets us use for loops on LinkedList
// Note: .into_iter() *consumes* underlying types
struct LinkedListIter {
    next_node: Option<LinkedList>,
}

// .iter() is not part of a standard rust trait
// it's a common and idiomatic "inherent impl" on many collection types
impl LinkedList {
    // Note: iter() takes a *reference* to a LinkedList
    // This is the fundamental distinction from .into_iter() that lets us iter over references
    fn iter(&self) -> LinkedListIterRef {
        LinkedListIterRef {
            next_node: Some(self),
        }
    }
}

// IntoIterator is a standard rust trait that lets us provide for loop functionality
// It has two associated types:
//  Item => the type returned by the iterator
//  IntoIter => the wrapper type implementing Iterator<Item = Self::Item>
impl IntoIterator for LinkedList {
    type Item = i32;
    type IntoIter = LinkedListIter;

    fn into_iter(self) -> Self::IntoIter {
        LinkedListIter {
            next_node: Some(self),
        }
    }
}

// This is the meat of our unconsumed iteration logic
// Because we're iterating over references we need explicit lifetype annotations
impl<'a> Iterator for LinkedListIterRef<'a> {
    type Item = &'a i32;

    fn next(&mut self) -> Option<Self::Item> {
        // Note: .map() is a very common idiom to work with Option types when:
        //  - when Option::Some, you need to operate on the value inside of option and get an Option
        //  - when Option::None, you need to get a None
        self.next_node.map(|node| {
            self.next_node = node.next.as_deref();
            // even though node is a &LinkedList, node.value is still an i32 (it's copied!)
            &node.value
        })
    }
}

impl Iterator for LinkedListIter {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        // We have to move the underlying LinkedList from next_node into node
        // We have to use take() here since next_node owns the LinkedList value
        // .take() is a common tool for getting an owned option from a shared reference
        // next_node is a shared reference to a LinkedList behind &mut self
        self.next_node.take().map(|node| {
            // The commented line below wouldn't work
            // as_deref() returns a reference to the underling LinkedList but does not move
            // ownership out of the Box - the Box still lives!
            // Since the Box still lives and owns the LinkedList it's illegal to try to move
            // *ll_ref to self.next_node
            // self.next_node = node.next.as_deref().map(|ll_ref| *ll_ref);
            self.next_node = node.next.map(|bx| *bx); // this works since bx owns the LinkedList
            node.value
        })
    }
}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn can_construct_linked_list() {
        let _ = LinkedList {
            value: 0,
            next: Some(Box::new(LinkedList {
                value: 1,
                next: None,
            })),
        };
    }

    #[test]
    fn can_into_iter_list() {
        let ll = LinkedList {
            value: 0,
            next: Some(Box::new(LinkedList {
                value: 1,
                next: None,
            })),
        };

        let v: Vec<i32> = ll.into_iter().collect();
        assert_eq!(vec![0, 1], v);
    }

    #[test]
    fn can_iter_list() {
        let ll = LinkedList {
            value: 0,
            next: Some(Box::new(LinkedList {
                value: 1,
                next: None,
            })),
        };

        let v1: Vec<&i32> = ll.iter().collect();
        let v2: Vec<&i32> = ll.iter().collect();
        assert_eq!(vec![&0, &1], v1);
        assert_eq!(vec![&0, &1], v2);
    }
}
