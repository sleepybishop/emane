all-local:
	cd $(top_srcdir)/rust/emane-core && cargo build
	cp $(top_srcdir)/rust/target/debug/emaneeventservice ./emaneeventservice

clean-local:
	rm -f emaneeventservice

install-exec-local:
	$(MKDIR_P) $(DESTDIR)$(bindir)
	$(INSTALL_PROGRAM) emaneeventservice $(DESTDIR)$(bindir)/emaneeventservice
