use std::iter::{DoubleEndedIterator, FromIterator, FusedIterator, IntoIterator};
use std::ops::{Index, Mul};
use std::rc::Rc;

#[derive(Clone, Debug, Copy)]
enum Color {
    Red,
    Black,
}
use Color::{Black, Red};

#[derive(Debug)]
enum Node<T> {
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
use Node::{Leaf, Tree};
impl<T: Clone> Node<T> {
    fn new(color: Color, left: Rc<Node<T>>, right: Rc<Node<T>>) -> Self {
        Tree {
            color,
            rank: left.rank()
                + match left.color() {
                    Black => 1,
                    Red => 0,
                },
            len: left.len() + right.len(),
            left,
            right,
        }
    }
    fn color(&self) -> Color {
        match self {
            Leaf { .. } => Black,
            Tree { color, .. } => *color,
        }
    }
    fn rank(&self) -> usize {
        match self {
            Leaf { .. } => 0,
            Tree { rank, .. } => *rank,
        }
    }
    fn len(&self) -> usize {
        match self {
            Leaf { .. } => 1,
            Tree { len, .. } => *len,
        }
    }
    fn left(&self) -> &Rc<Node<T>> {
        match self {
            Leaf { .. } => unreachable!(),
            Tree { left, .. } => left,
        }
    }
    fn right(&self) -> &Rc<Node<T>> {
        match self {
            Leaf { .. } => unreachable!(),
            Tree { right, .. } => right,
        }
    }
    fn index(&self, index: usize) -> &T {
        match self {
            Leaf { val } => val,
            Tree { left, right, .. } => {
                if index < left.len() {
                    left.index(index)
                } else {
                    right.index(index - left.len())
                }
            }
        }
    }
    fn to_black(src: &Rc<Self>) -> Rc<Self> {
        match src.color() {
            Red => Rc::new(Self::new(
                Black,
                Rc::clone(src.left()),
                Rc::clone(src.right()),
            )),
            Black => Rc::clone(src),
        }
    }
    fn merge(left: &Rc<Self>, right: &Rc<Self>) -> Rc<Self> {
        Rc::new(if left.rank() < right.rank() {
            let left = &Node::merge(left, right.left());
            match (left.color(), left.left().color(), right.color()) {
                (Red, Red, Black) => match right.right().color() {
                    Black => Self::new(
                        Black,
                        Rc::clone(left.left()),
                        Rc::new(Self::new(
                            Red,
                            Rc::clone(left.right()),
                            Rc::clone(right.right()),
                        )),
                    ),
                    Red => Self::new(
                        Red,
                        Rc::new(Self::new(
                            Black,
                            Rc::clone(left.left()),
                            Rc::clone(left.right()),
                        )),
                        Rc::new(Self::new(
                            Black,
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
                (Black, Red, Red) => match left.left().color() {
                    Black => Self::new(
                        Black,
                        Rc::new(Self::new(
                            Red,
                            Rc::clone(left.left()),
                            Rc::clone(right.left()),
                        )),
                        Rc::clone(right.right()),
                    ),
                    Red => Self::new(
                        Red,
                        Rc::new(Self::new(
                            Black,
                            Rc::clone(left.left().left()),
                            Rc::clone(left.left().right()),
                        )),
                        Rc::new(Self::new(
                            Black,
                            Rc::clone(right.left()),
                            Rc::clone(right.right()),
                        )),
                    ),
                },
                _ => Self::new(left.color(), Rc::clone(left.left()), Rc::clone(right)),
            }
        } else {
            Self::new(Red, Rc::clone(left), Rc::clone(right))
        })
    }
    fn split(tree: &Rc<Self>, index: usize) -> (Rc<Self>, Rc<Self>) {
        match tree.as_ref() {
            Tree { left, right, .. } => {
                if index < left.len() {
                    let (left_left, left_right) = Self::split(left, index);
                    (left_left, Self::to_black(&Self::merge(&left_right, right)))
                } else if index > left.len() {
                    let (right_left, right_right) = Self::split(right, index - left.len());
                    (Self::to_black(&Self::merge(left, &right_left)), right_right)
                } else {
                    (Self::to_black(left), Self::to_black(right))
                }
            }
            _ => unreachable!(),
        }
    }
}
impl<T: Clone + Mul> Node<T> {}

#[derive(Clone, Debug)]
pub struct PersistentLazyRBTree<T> {
    root: Option<Rc<Node<T>>>,
}
impl<T: Clone> PersistentLazyRBTree<T> {
    fn from(root: Rc<Node<T>>) -> Self {
        Self { root: Some(root) }
    }
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
            (Some(left), Some(right)) => Self::from(Node::to_black(&Node::merge(left, right))),
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
            (Self::from(left), Self::from(right))
        }
    }
    pub fn insert(&self, index: usize, val: T) -> Self {
        assert!(index <= self.len());
        let (ref left, ref right) = self.split(index);
        Self::merge(
            &Self::merge(left, &Self::from(Rc::new(Leaf { val }))),
            right,
        )
    }
    pub fn erase(&self, index: usize) -> Self {
        assert!(index < self.len());
        let (ref left, ref right) = self.split(index);
        let (_, ref right) = right.split(1);
        Self::merge(left, right)
    }
    pub fn iter(&self) -> Iter<T> {
        Iter {
            begin: 0,
            end: self.len(),
            tree: self,
        }
    }
}

impl<T: Clone> Index<usize> for PersistentLazyRBTree<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        self.root.as_ref().unwrap().index(index)
    }
}
pub struct Iter<'a, T: 'a> {
    begin: usize,
    end: usize,
    tree: &'a PersistentLazyRBTree<T>,
}
impl<'a, T: Clone> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.begin < self.tree.len() {
            let ret = Some(&self.tree[self.begin]);
            self.begin += 1;
            ret
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.tree.len(), Some(self.tree.len()))
    }
}
impl<'a, T: Clone> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.tree.len()
    }
}
impl<'a, T: Clone> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end > 0 {
            self.end -= 1;
            Some(&self.tree[self.end])
        } else {
            None
        }
    }
}
impl<'a, T: Clone> FusedIterator for Iter<'a, T> {}
impl<T: Clone> FromIterator<T> for PersistentLazyRBTree<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut res: Vec<Self> = vec![];
        for item in iter {
            let mut cur = Self::new().insert(0, item);
            while let Some(last) = res.last() {
                if last.len() != cur.len() {
                    break;
                }
                cur = Self::merge(last, &cur);
                res.pop();
            }
            res.push(cur);
        }
        while res.len() >= 2 {
            let right = res.pop().unwrap();
            let left = res.pop().unwrap();
            res.push(Self::merge(&left, &right));
        }
        res.remove(0)
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
