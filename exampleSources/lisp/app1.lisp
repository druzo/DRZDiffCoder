;; Common Lisp — defun with recursion over a list.

(defun task-priority (task) (getf task :priority))
(defun task-title    (task) (getf task :title))

(defun sort-by-priority (tasks)
  (sort (copy-list tasks)
        #'<=
        :key #'task-priority))

(defun main ()
  (let ((backlog (list (list :title "Write tests"     :priority 2)
                       (list :title "Fix login bug"   :priority 5)
                       (list :title "Refactor parser" :priority 3))))
    (dolist (task (sort-by-priority backlog))
      (format t "~A  ~A~%"
              (task-priority task)
              (task-title    task)))))

(main)