use std::rc::Rc;

#[derive(Clone, Debug, Copy)]
enum Color {
    Red,
    Black,
}

#[derive(Debug)]
enum Node<T: Clone> {
    Leaf {
        val: T,
    },
    Tree {
        color: Color,
        rank: usize,
        len: usize,
        left: Rc<Node<T>>,
        right: Rc<Node<T>>,
    },
}
impl<T: Clone> Node<T> {
    fn new(color: Color, left: Rc<Node<T>>, right: Rc<Node<T>>) -> Self {
        Node::Tree {
            color,
            rank: left.rank()
                + match left.color() {
                    Color::Black => 1,
                    Color::Red => 0,
                },
            len: left.len() + right.len(),
            left,
            right,
        }
    }
    fn color(&self) -> Color {
        match self {
            Node::Leaf { .. } => Color::Black,
            Node::Tree { color, .. } => *color,
        }
    }
    fn rank(&self) -> usize {
        match self {
            Node::Leaf { .. } => 0,
            Node::Tree { rank, .. } => *rank,
        }
    }
    fn len(&self) -> usize {
        match self {
            Node::Leaf { .. } => 1,
            Node::Tree { len, .. } => *len,
        }
    }
    fn left(&self) -> &Rc<Node<T>> {
        match self {
            Node::Leaf { .. } => unreachable!(),
            Node::Tree { left, .. } => left,
        }
    }
    fn right(&self) -> &Rc<Node<T>> {
        match self {
            Node::Leaf { .. } => unreachable!(),
            Node::Tree { right, .. } => right,
        }
    }
    fn index(&self, index: usize) -> &T {
        match self {
            Node::Leaf { val } => val,
            Node::Tree { left, right, .. } => {
                if index < left.len() {
                    left.index(index)
                } else {
                    right.index(index - left.len())
                }
            }
        }
    }
    fn merge(left: &Rc<Self>, right: &Rc<Self>) -> Rc<Self> {
        Rc::new(if left.rank() < right.rank() {
            let left = &Node::merge(left, right.left());
            match (left.color(), left.left().color(), right.color()) {
                (Color::Red, Color::Red, Color::Black) => match right.right().color() {
                    Color::Black => Self::new(
                        Color::Black,
                        Rc::clone(left.left()),
                        Rc::new(Self::new(
                            Color::Red,
                            Rc::clone(left.right()),
                            Rc::clone(right.right()),
                        )),
                    ),
                    Color::Red => Self::new(
                        Color::Red,
                        Rc::new(Self::new(
                            Color::Black,
                            Rc::clone(left.left()),
                            Rc::clone(left.right()),
                        )),
                        Rc::new(Self::new(
                            Color::Black,
                            Rc::clone(right.right().left()),
                            Rc::clone(right.right().right()),
                        )),
                    ),
                },
                _ => Self::new(right.color(), Rc::clone(left), Rc::clone(right.right())),
            }
        } else if left.rank() > right.rank() {
            let right = &Node::merge(left.right(), right);
            match (left.color(), right.right().color(), right.color()) {
                (Color::Black, Color::Red, Color::Red) => match left.left().color() {
                    Color::Black => Self::new(
                        Color::Black,
                        Rc::new(Self::new(
                            Color::Red,
                            Rc::clone(left.left()),
                            Rc::clone(right.left()),
                        )),
                        Rc::clone(right.right()),
                    ),
                    Color::Red => Self::new(
                        Color::Red,
                        Rc::new(Self::new(
                            Color::Black,
                            Rc::clone(left.left().left()),
                            Rc::clone(left.left().right()),
                        )),
                        Rc::new(Self::new(
                            Color::Black,
                            Rc::clone(right.left()),
                            Rc::clone(right.right()),
                        )),
                    ),
                },
                _ => Self::new(left.color(), Rc::clone(left.left()), Rc::clone(right)),
            }
        } else {
            Self::new(Color::Red, Rc::clone(left), Rc::clone(right))
        })
    }
    fn split(tree: &Rc<Self>, index: usize) -> (Rc<Self>, Rc<Self>) {
        match &**tree {
            Node::Tree { left, right, .. } => {
                if index < left.len() {
                    let (left_left, left_right) = Self::split(left, index);
                    (left_left, Self::merge(&left_right, right))
                } else if index > left.len() {
                    let (right_left, right_right) = Self::split(right, index - left.len());
                    (Self::merge(left, &right_left), right_right)
                } else {
                    (Rc::clone(left), Rc::clone(right))
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PersistentLazyRBTree<T: Clone> {
    root: Option<Rc<Node<T>>>,
}
impl<T: Clone> PersistentLazyRBTree<T> {
    pub fn new() -> Self {
        Self { root: None }
    }
    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }
    pub fn merge(left: &Self, right: &Self) -> Self {
        match (&left.root, &right.root) {
            (None, _) => right.clone(),
            (_, None) => left.clone(),
            (Some(left), Some(right)) => {
                let root = Node::merge(left, right);
                Self {
                    root: Some(Rc::new(Node::new(
                        Color::Black,
                        Rc::clone(root.left()),
                        Rc::clone(root.right()),
                    ))),
                }
            }
        }
    }
    pub fn split(&self, index: usize) -> (Self, Self) {
        assert!(index <= self.len());
        if index == 0 {
            (Self::new(), self.clone())
        } else if index == self.len() {
            (self.clone(), Self::new())
        } else {
            let (left, right) = Node::split(self.root.as_ref().unwrap(), index);
            (Self { root: Some(left) }, Self { root: Some(right) })
        }
    }
    pub fn insert(&self, index: usize, val: T) -> Self {
        assert!(index <= self.len());
        let (ref left, ref right) = self.split(index);
        Self::merge(
            &Self::merge(
                left,
                &Self {
                    root: Some(Rc::new(Node::Leaf { val })),
                },
            ),
            right,
        )
    }
    pub fn erase(&self, index: usize) -> Self {
        assert!(index < self.len());
        let (ref left, ref right) = self.split(index);
        let (_, ref right) = right.split(1);
        Self::merge(left, right)
    }
}

use std::ops::Index;
impl<T: Clone> Index<usize> for PersistentLazyRBTree<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        self.root.as_ref().unwrap().index(index)
    }
}

#[cfg(test)]
mod tests {
    use crate::PersistentLazyRBTree;
    use rand::Rng;
    #[test]
    fn it_works() {
        let mut rng = rand::thread_rng();
        let mut vec = Vec::new();
        let mut rbtree = PersistentLazyRBTree::new();
        let n = 100000;
        for _ in 0..n {
            let x: i64 = rng.gen();
            vec.push(x);
            rbtree = rbtree.insert(rbtree.len(), x);
        }
        let q = 100000;
        for _ in 0..q {
            let x = rng.gen();
            let i = rng.gen_range(0, vec.len() + 1);
            vec.insert(i, x);
            rbtree = rbtree.insert(i, x);

            let i = rng.gen_range(0, vec.len());
            vec.remove(i);
            rbtree = rbtree.erase(i);

            let i = rng.gen_range(0, vec.len());
            assert_eq!(vec[i], rbtree[i]);
        }
    }
}
