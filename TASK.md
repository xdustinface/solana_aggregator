# Home Case Study
**Case Study: Building a Solana Blockchain Data Aggregator**
### Context:
You are assigned to develop a mini Data aggregator software that collects and processes data from the Solana blockchain. The goal is to create a system capable of retrieving transaction and account data for the ongoing epoch.
### Our Goal:
Keep in mind that we are not expecting you to create a production-ready software (docker, ci, prometheus metrics…) but to understand your level in Rust and the basics of system design. We are still expecting the code to be somehow clean as it will reflect you Rust level (eg. handling errors, decent project composition, etc).
### Resources:
```
💡
- To get a node on devnet, you can go to https://www.helius.dev/ and create a free account.
- You can also use the official solana testnet.
```
These are examples, you can choose the provider of your choice.
### Requirements:
1. **Data Retrieval:** Develop a Rust application capable of retrieving transaction and account data from the Solana blockchain on devnet or testnet. Utilise Solana's API or SDK to interact with the blockchain and fetch relevant data.
2. **Data Processing:** Implement mechanisms to process the retrieved data efficiently. This includes parsing transaction records, extracting relevant information such as sender, receiver, amount, timestamp, etc., and organising data into a structured format for further analysis and queries.
3. **Data History:** Configure the data aggregator to start aggregating data from the current epoch and onwards. Exclude historical data to focus on recent transactions and account changes. Ensure the data aggregator provides real-time updates by continuously monitoring the blockchain for new transactions and account changes.
4. **Data Storage (optional):** Choose a storage solution to store the collected data securely. Consider using a suitable database or data storage mechanism that offers scalability, reliability, and fast query capabilities. If you are running out of time, a in-memory structure is enough!
5. **API Integration:** Create a RESTful API layer to expose the aggregated data to external systems and applications. The API should support various queries to retrieve transaction history, account details, and other relevant information.

```💡
route (transactions)
1. transactions/?id=4CqYTMNtGpWjk67Ntq9QtDHZNaDeqYwhbh6cMVx7Qx6Y4b43kgsHP8t4TJbdrWf5kD4xuWNXhFLZfo4H6GBmxXzG
2. transactions/?day=23/05/2023
```
### Deliverables:
1. **Data Aggregator Application:** Develop a Rust-based application that fulfils the requirements outlined above. The application should be well-structured, modular, and decently documented to facilitate the review and highlight some enhancements. The aggregator and the API should be run with the same command!
2. **Documentation:** Provide in the README a few notes covering the architecture, design decisions, API endpoints, usage instructions, and any other relevant information necessary for deploying and using the data aggregator tool.
3. **Testing:** Write tests to ensure the reliability, performance, and security of the data aggregator and API.
### Evaluation Criteria:
- **Functionality:** Does the data aggregator retrieve and process Solana blockchain data accurately and efficiently?
- **Performance:** How well does the application handle large volumes of data and concurrent requests?
- **Reliability:** Is the data aggregator resilient to failures and capable of recovering gracefully?
- **Scalability:** Can the application scale to handle increasing data loads without sacrificing performance?
- **Security:** Are proper security measures implemented to protect data integrity?
- **Documentation and Maintainability:** Is the codebase well-documented, well-composed, maintainable, and easy to understand for future developers?
