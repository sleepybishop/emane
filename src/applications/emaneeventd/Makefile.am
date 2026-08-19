all-local:
	cd $(top_srcdir)/rust/emane-core && cargo build
	cp $(top_srcdir)/rust/target/debug/emaneeventd ./emaneeventd

clean-local:
	rm -f emaneeventd

install-exec-local:
	$(MKDIR_P) $(DESTDIR)$(bindir)
	$(INSTALL_PROGRAM) emaneeventd $(DESTDIR)$(bindir)/emaneeventd
