(defvar straight-utils-install-buffer-name " *Install straight-utils* ")

(defun straight-utils-module--cargo-is-available ()
  (unless (executable-find "cargo")
    (error "Straight-Utils needs Rust to be compiled.  Please, install Rust"))
  t)

;;;###autoload
(defun straight-utils-module-compile ()
  "Compile straight-utils-module."
  (interactive)
  (when (straight-utils-module--cargo-is-available)
    (let* ((straight-utils-directory
            (shell-quote-argument
             (file-name-directory
	      (locate-library "straight-utils.el" t))))
           (build-commands
            (format
	     "cd %s; cargo build --release; cp target/release/libstraight_utils_module.so straight-utils-module.so; cd -" straight-utils-directory))
           (buffer
	    (get-buffer-create straight-utils-install-buffer-name)))
      (pop-to-buffer buffer)
      (compilation-mode)
      (if (zerop
	   (let ((inhibit-read-only t))
             (call-process "sh" nil buffer t "-c" build-commands)))
          (message "Compilation of `straight-utils-module' succeeded")
        (error "Compilation of `straight-utils-module' module failed!")))))

(unless (require 'straight-utils-module nil t)
  (if (y-or-n-p "Straight-Utils needs `straight-utils-module' to work.  Compile it now? ")
      (progn
	(straight-utils-module-compile)
	(require 'straight-utils-module))
    (error "Straight-Utils will not work until `straight-utils-module' is compiled!")))


(provide 'straight-utils)
