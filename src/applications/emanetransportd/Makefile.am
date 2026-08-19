all-local:
	cd $(top_srcdir)/rust/emane-core && cargo build
	cp $(top_srcdir)/rust/target/debug/emanetransportd ./emanetransportd

clean-local:
	rm -f emanetransportd

install-exec-local:
	$(MKDIR_P) $(DESTDIR)$(bindir)
	$(INSTALL_PROGRAM) emanetransportd $(DESTDIR)$(bindir)/emanetransportd
