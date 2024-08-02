# Rust Motion Detection through multi frame diff

This little project was inspired by a [Posy Video](https://www.youtube.com/watch?v=NSS6yAMZF78) which talks about motion extraction by offsetting the video and subtracting it from the original to emphasize motion. After realizing that given a webcam feed I could just save the last captured frame and run the same math against that. This is the outcome of that. A little Rust based application that renders the video feed of your camera into a little window. It is by no means optimal in any regard but it proves that the base concept works.

Next on the todo is add bounding boxes around moving objects based on their now clear outline.
