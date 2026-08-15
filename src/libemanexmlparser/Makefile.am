lib_LTLIBRARIES = libemanexmlparser.la

libemanexmlparser_la_CPPFLAGS= \
 -I@top_srcdir@/include        \
 $(AM_CPPFLAGS)

libemanexmlparser_la_DEPENDENCIES = $(top_srcdir)/target/debug/libemane_core.a

libemanexmlparser_la_LIBADD= \
 $(top_srcdir)/target/debug/libemane_core.a

libemanexmlparser_la_LDFLAGS= \
 $(AM_LDFLAGS)                \
 -avoid-version

libemanexmlparser_la_SOURCES=      \
 configurationparser.cc            \
 eventagentconfiguration.cc        \
 eventdaemonconfiguration.cc       \
 eventgeneratorconfiguration.cc    \
 eventserviceconfiguration.cc      \
 layerconfiguration.cc             \
 nemconfiguration.cc               \
 platformconfiguration.cc          \
 pluginconfiguration.cc            \
 transportdaemonconfiguration.cc   \
 transportinstanceconfiguration.cc \
 configurationparser.h            \
 eventagentconfiguration.h        \
 eventdaemonconfiguration.h       \
 eventgeneratorconfiguration.h    \
 emaneparseexception.h            \
 eventserviceconfiguration.h      \
 layerconfiguration.h             \
 nemconfiguration.h               \
 platformconfiguration.h          \
 pluginconfiguration.h            \
 transportdaemonconfiguration.h   \
 transportinstanceconfiguration.h

