CC = gcc
CFLAGS = -Wall -Wextra -O2 $(shell pkg-config --cflags libusb-1.0 2>/dev/null || echo -I/usr/include/libusb-1.0)
LIBS = $(shell pkg-config --libs libusb-1.0 2>/dev/null || echo -lusb-1.0) -lm

all: flirone

flirone: src/flirone.c src/plank.h src/font5x7.h
	$(CC) $(CFLAGS) -o flirone src/flirone.c $(LIBS)

clean:
	rm -f flirone
