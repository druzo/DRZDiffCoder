;; Common Lisp — mapcar + lambda + reduce.

(defun describe (task)
  (format nil "~A  ~A" (getf task :priority) (getf task :title)))

(defun open-titles (backlog)
  (mapcar (lambda (t) (getf t :title))
          (remove-if-not (lambda (t) (eq (getf t :status) :open)) backlog)))

(defun sum-priorities (backlog)
  (reduce #'+ backlog :key (lambda (t) (getf t :priority))))

(defun main ()
  (let ((backlog (list (list :title "Write tests"     :priority 2 :status :open)
                       (list :title "Fix login bug"   :priority 5 :status :blocked)
                       (list :title "Refactor parser" :priority 3 :status :done))))
    (format t "open titles: ~{~A~^, ~}~%" (open-titles backlog))
    (format t "sum         : ~A~%" (sum-priorities backlog))))

(main)