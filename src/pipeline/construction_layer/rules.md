* a link can either be critical or non-critical
* critical links:
  * The receiver on a critical link always expects a value and will error if it doesnt get one (timeout)
  * if a critical link fails, all immediate successors should be stopped with the correct error message
  * The sender on a critical link should always block until the receiver is willing to receive
* Non critical links:
  * if there is only a single link defined as non-critical, 
  it behaves as a critical link since every node needs at least one input (except sources and sinks)
  * the receiver of a non critical link that has multiple inputs populates a default value for inputs that are not sending
  * the sender into a non critical input should not block
  * should only call a node on a non-critical link after predecessor has been called to avoid lockups
* all linear links are critical. 
* All multiplexed links are non-critical to avoid errors
* all branch and joint links can be either critical or non critical
* feedback links should be critical

* every communicator should have attached metadata that defines if it is critical or non critical
* should also store metadata about the source name so that a graph can be reliably constructed from the nodes in the pipeline
* for series send systems, everything after a splitter and before a reconstructor, for splitter into n items, every standard receiver must be run n times per run of the splitter

* handling of receive types is done internal to th enode, user defined

now with this in mind. How is the pipeline construction modified?
* every builder method takes critical or noncritical
* metadata is constructed internally for each receiver

# On determining whether a node should be run or not given proper ordering
* the main dispatcher must be running in a loop to reduce latency
* 